use crate::error::ApiError;
use axum::http::{HeaderMap, header};
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedDeployment {
    pub deployment_instance_id: i64,
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
    use super::hash_bearer_token;

    #[test]
    fn hashes_bearer_token_to_sha256_hex() {
        assert_eq!(
            hash_bearer_token("mini-conf-test-token"),
            "2b9a8821c9539e3376dc45b347ab94f8e3691c9a03485f91343393b5e402cc51"
        );
    }
}
