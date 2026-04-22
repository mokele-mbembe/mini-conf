use crate::error::ApiError;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::http::{HeaderMap, HeaderValue, header};
use axum_extra::extract::cookie::{Cookie, SameSite};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

const ADMIN_SESSION_COOKIE: &str = "mini_conf_session";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedDeployment {
    pub deployment_instance_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub username: String,
    pub is_platform_admin: bool,
    pub status: String,
    pub must_change_password: bool,
}

pub async fn authenticate_open_request(
    pool: &sqlx::PgPool,
    headers: &HeaderMap,
) -> Result<AuthenticatedDeployment, ApiError> {
    let token = bearer_token(headers)?;
    let token_hash = hash_bearer_token(token);

    let row = sqlx::query(
        r#"
        SELECT id, deployment_instance_id
        FROM deployment_credentials
        WHERE token_hash = $1
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::unauthorized("invalid_token", "Invalid deployment token"))?;

    let credential_id: i64 = row.get("id");
    let deployment_instance_id: i64 = row.get("deployment_instance_id");

    sqlx::query(
        r#"
        UPDATE deployment_credentials
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(credential_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(AuthenticatedDeployment {
        deployment_instance_id,
    })
}

pub fn ensure_deployment_access(
    auth: AuthenticatedDeployment,
    deployment_instance_id: i64,
) -> Result<(), ApiError> {
    if auth.deployment_instance_id != deployment_instance_id {
        return Err(ApiError::forbidden(
            "deployment_forbidden",
            "Deployment token does not match target deployment",
        ));
    }

    Ok(())
}

pub fn hash_bearer_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);

    for byte in digest {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }

    output
}

pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(password_hash).map_err(|_| ApiError::internal())?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn generate_session_token() -> String {
    Uuid::new_v4().to_string()
}

pub fn generate_deployment_token() -> String {
    format!("mc_live_{}", Uuid::new_v4().simple())
}

pub fn deployment_token_preview(token: &str) -> String {
    if token.starts_with("mc_live_") {
        "mc_live_***".to_owned()
    } else {
        "***".to_owned()
    }
}

pub fn session_cookie_header(token: &str) -> HeaderValue {
    let cookie = Cookie::build((ADMIN_SESSION_COOKIE, token.to_owned()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

    HeaderValue::from_str(&cookie.encoded().to_string())
        .expect("session cookie should always produce a valid header")
}

pub fn clear_session_cookie_header() -> HeaderValue {
    HeaderValue::from_static("mini_conf_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

pub async fn authenticate_admin_session(
    pool: &sqlx::PgPool,
    cookie_header: Option<&str>,
) -> Result<AuthenticatedUser, ApiError> {
    let token = session_token(cookie_header).ok_or_else(|| {
        ApiError::unauthorized("auth_session_expired", "Authentication session expired")
    })?;
    let token_hash = hash_bearer_token(token);

    let row = sqlx::query(
        r#"
                SELECT s.id, u.id AS user_id, u.username, u.is_platform_admin, u.status, u.must_change_password
        FROM auth_sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.session_token_hash = $1
          AND s.status = 'active'
          AND s.expires_at > NOW()
          AND u.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::unauthorized("auth_session_expired", "Authentication session expired")
    })?;

    let session_id: i64 = row.get("id");
    let user_id: i64 = row.get("user_id");
    let username: String = row.get("username");
    let is_platform_admin: bool = row.get("is_platform_admin");
    let status: String = row.get("status");
    let must_change_password: bool = row.get("must_change_password");

    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(AuthenticatedUser {
        user_id,
        username,
        is_platform_admin,
        status,
        must_change_password,
    })
}

pub async fn revoke_admin_session(
    pool: &sqlx::PgPool,
    cookie_header: Option<&str>,
) -> Result<(), ApiError> {
    let Some(token) = session_token(cookie_header) else {
        return Ok(());
    };

    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET status = 'revoked', updated_at = NOW()
        WHERE session_token_hash = $1
        "#,
    )
    .bind(hash_bearer_token(token))
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(())
}

fn session_token(cookie_header: Option<&str>) -> Option<&str> {
    let cookie_header = cookie_header?;
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix(&format!("{ADMIN_SESSION_COOKIE}="))
            && !value.trim().is_empty()
        {
            return Some(value);
        }
    }

    None
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("nibble should always be within 0..=15"),
    }
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("missing_token", "Missing Bearer token"))?;
    let value = value
        .to_str()
        .map_err(|_| ApiError::unauthorized("invalid_token", "Invalid Bearer token"))?;

    let token = value
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("invalid_token", "Invalid Bearer token"))?;

    if token.trim().is_empty() {
        return Err(ApiError::unauthorized(
            "invalid_token",
            "Invalid Bearer token",
        ));
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::{
        bearer_token, clear_session_cookie_header, deployment_token_preview,
        generate_deployment_token, hash_bearer_token, session_cookie_header,
    };
    use axum::http::{HeaderMap, HeaderValue, header};

    #[test]
    fn hashes_bearer_token_to_sha256_hex() {
        assert_eq!(
            hash_bearer_token("mini-conf-test-token"),
            "2b9a8821c9539e3376dc45b347ab94f8e3691c9a03485f91343393b5e402cc51"
        );
    }

    #[test]
    fn generates_deployment_tokens_with_live_prefix() {
        let token = generate_deployment_token();

        assert!(token.starts_with("mc_live_"));
        assert!(token.len() > "mc_live_".len());
    }

    #[test]
    fn deployment_token_preview_masks_live_tokens() {
        assert_eq!(
            deployment_token_preview("mc_live_1234567890abcdef"),
            "mc_live_***"
        );
    }

    #[test]
    fn deployment_token_preview_masks_non_live_tokens_generically() {
        assert_eq!(deployment_token_preview("legacy-token"), "***");
        assert_eq!(deployment_token_preview(""), "***");
    }

    #[test]
    fn bearer_token_extracts_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer mc_live_token"),
        );

        assert_eq!(bearer_token(&headers), Ok("mc_live_token"));
    }

    #[test]
    fn bearer_token_rejects_missing_authorization_header() {
        let headers = HeaderMap::new();

        assert_eq!(
            bearer_token(&headers).map_err(|error| error.into_body().code),
            Err("missing_token".to_owned())
        );
    }

    #[test]
    fn bearer_token_rejects_missing_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Token mc_live_token"),
        );

        assert_eq!(
            bearer_token(&headers).map_err(|error| error.into_body().code),
            Err("invalid_token".to_owned())
        );
    }

    #[test]
    fn bearer_token_rejects_empty_token() {
        for raw in ["Bearer ", "Bearer    "] {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::AUTHORIZATION,
                HeaderValue::from_str(raw).expect("test header value should be valid"),
            );

            assert_eq!(
                bearer_token(&headers).map_err(|error| error.into_body().code),
                Err("invalid_token".to_owned())
            );
        }
    }

    #[test]
    fn session_cookie_header_sets_expected_security_attributes() {
        let value = session_cookie_header("session-token");
        let cookie = value.to_str().expect("session cookie should be ascii");

        assert!(cookie.contains("mini_conf_session=session-token"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
    }

    #[test]
    fn clear_session_cookie_header_expires_session_cookie() {
        let value = clear_session_cookie_header();
        let cookie = value.to_str().expect("clear cookie should be ascii");

        assert!(cookie.contains("mini_conf_session="));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Lax"));
        assert!(cookie.contains("Max-Age=0"));
    }
}
