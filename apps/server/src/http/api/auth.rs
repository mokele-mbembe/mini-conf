use crate::{
    audit::{AuditLogEntry, write_audit_log, write_audit_log_best_effort},
    auth::{
        CSRF_HEADER_NAME, authenticate_admin_session, clear_csrf_cookie_header,
        clear_session_cookie_header, csrf_cookie_header, csrf_header_value, csrf_token,
        generate_csrf_token, generate_session_token, hash_bearer_token, hash_password,
        revoke_admin_session, session_cookie_header, validate_password_strength, verify_password,
    },
    error::ApiError,
    security::login_throttle_key,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use schema::auth::{AuthSessionResponse, AuthUser};
use serde::Deserialize;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct LoginRequest {
    username: Option<String>,
    password: Option<String>,
}

#[derive(Debug)]
struct ValidatedLoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChangePasswordRequest {
    current_password: Option<String>,
    new_password: Option<String>,
}

#[derive(Debug)]
struct ValidatedChangePasswordRequest {
    current_password: String,
    new_password: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/csrf", get(get_csrf))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/change-password", post(change_password))
}

#[utoipa::path(
    get,
    path = "/api/auth/csrf",
    tag = "auth",
    responses(
        (status = 204, description = "CSRF token issued", headers(("set-cookie" = String, description = "Readable CSRF cookie"), ("x-csrf-token" = String, description = "CSRF token for cross-origin clients"))),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_csrf(State(state): State<AppState>) -> Result<Response, ApiError> {
    if state.db_pool().is_none() {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    }

    let csrf_token = generate_csrf_token();
    let mut response = StatusCode::NO_CONTENT.into_response();
    append_csrf_token(&mut response, &csrf_token, &state)?;
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = crate::openapi::LoginRequestBody,
    responses(
        (status = 200, description = "Login successful and session cookie issued", body = AuthSessionResponse, headers(("set-cookie" = String, description = "HttpOnly session cookie"))),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 403, description = "Missing or invalid CSRF token", body = crate::error::ErrorResponse),
        (status = 401, description = "Invalid username or password", body = crate::error::ErrorResponse),
        (status = 429, description = "Too many failed login attempts", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<LoginRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    require_login_csrf(&headers)?;
    let throttle_key = login_throttle_key(&headers, &payload.username);
    state.login_throttle().ensure_allowed(&throttle_key)?;

    let row = sqlx::query(
        r#"
                SELECT id, username, password_hash, is_platform_admin, status, must_change_password
        FROM users
        WHERE username = $1
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(&payload.username)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to fetch login user"))?;
    let Some(row) = row else {
        write_audit_log_best_effort(
            pool,
            AuditLogEntry {
                project_id: None,
                user_id: None,
                action: "auth.login_failed",
                resource_type: "auth",
                resource_id: payload.username.clone(),
                detail: Some(serde_json::json!({
                    "username": payload.username
                })),
            },
        )
        .await;
        state.login_throttle().record_failure(&throttle_key);
        return Err(ApiError::unauthorized(
            "invalid_credentials",
            "Invalid username or password",
        ));
    };

    let password_hash: String = row.get("password_hash");
    if !verify_password(&payload.password, &password_hash)? {
        let user_id: i64 = row.get("id");
        let username: String = row.get("username");
        write_audit_log_best_effort(
            pool,
            AuditLogEntry {
                project_id: None,
                user_id: Some(user_id),
                action: "auth.login_failed",
                resource_type: "auth",
                resource_id: username.clone(),
                detail: Some(serde_json::json!({
                    "username": username
                })),
            },
        )
        .await;
        state.login_throttle().record_failure(&throttle_key);
        return Err(ApiError::unauthorized(
            "invalid_credentials",
            "Invalid username or password",
        ));
    }

    let user_id: i64 = row.get("id");
    let username: String = row.get("username");
    let is_platform_admin: bool = row.get("is_platform_admin");
    let status: String = row.get("status");
    let must_change_password: bool = row.get("must_change_password");
    let session_token = generate_session_token();
    let csrf_token = generate_csrf_token();
    state.login_throttle().record_success(&throttle_key);

    sqlx::query(
        r#"
        UPDATE users
        SET last_login_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to update user login timestamp"))?;

    sqlx::query(
        r#"
        INSERT INTO auth_sessions (user_id, session_token_hash, expires_at)
        VALUES ($1, $2, NOW() + INTERVAL '7 days')
        "#,
    )
    .bind(user_id)
    .bind(hash_bearer_token(&session_token))
    .execute(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to create auth session"))?;
    write_audit_log_best_effort(
        pool,
        AuditLogEntry {
            project_id: None,
            user_id: Some(user_id),
            action: "auth.login_success",
            resource_type: "auth",
            resource_id: username.clone(),
            detail: Some(serde_json::json!({
                "username": username
            })),
        },
    )
    .await;

    let mut response = Json(AuthSessionResponse {
        user: AuthUser {
            id: user_id,
            username,
            is_platform_admin,
            status,
            must_change_password,
        },
        auth_mode: "session".to_owned(),
    })
    .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        session_cookie_header(
            &session_token,
            state.config().session_cookie_secure,
            state.config().session_cookie_same_site,
        )?,
    );
    append_csrf_token(&mut response, &csrf_token, &state)?;
    Ok(response)
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Current authenticated admin session", body = AuthSessionResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let auth = authenticate_admin_session(pool, cookie_header).await?;

    let csrf = csrf_token(cookie_header)
        .map(str::to_owned)
        .unwrap_or_else(generate_csrf_token);

    let mut response = Json(AuthSessionResponse {
        user: AuthUser {
            id: auth.user_id,
            username: auth.username,
            is_platform_admin: auth.is_platform_admin,
            status: auth.status,
            must_change_password: auth.must_change_password,
        },
        auth_mode: "session".to_owned(),
    })
    .into_response();
    append_csrf_token(&mut response, &csrf, &state)?;
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/auth/change-password",
    tag = "auth",
    request_body = schema::auth::ChangePasswordRequest,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Password changed and current session refreshed", body = AuthSessionResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing, expired, or invalid current password", body = crate::error::ErrorResponse),
        (status = 422, description = "Password does not meet strength requirements", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn change_password(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<ChangePasswordRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let auth = authenticate_admin_session(pool, cookie_header).await?;

    validate_password_strength(&payload.new_password)?;
    let row = sqlx::query(
        r#"
        SELECT password_hash
        FROM users
        WHERE id = $1
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to fetch current password hash"))?;
    let password_hash: String = row.get("password_hash");

    if !verify_password(&payload.current_password, &password_hash)? {
        return Err(ApiError::unauthorized(
            "current_password_invalid",
            "Current password is invalid",
        ));
    }

    let new_password_hash = hash_password(&payload.new_password)?;
    let mut tx = pool.begin().await.map_err(|error| {
        ApiError::internal_with(error, "failed to start password change transaction")
    })?;
    sqlx::query(
        r#"
        UPDATE users
        SET
            password_hash = $2,
            must_change_password = FALSE,
            password_updated_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(auth.user_id)
    .bind(new_password_hash)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to update user password"))?;

    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET status = 'revoked', updated_at = NOW()
        WHERE user_id = $1
          AND id <> $2
          AND status = 'active'
        "#,
    )
    .bind(auth.user_id)
    .bind(auth.session_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to revoke other auth sessions"))?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(auth.user_id),
            action: "auth.password_changed",
            resource_type: "auth",
            resource_id: auth.user_id.to_string(),
            detail: Some(serde_json::json!({
                "changed_fields": ["password", "must_change_password"]
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|error| {
        ApiError::internal_with(error, "failed to commit password change transaction")
    })?;

    let csrf = csrf_token(cookie_header)
        .map(str::to_owned)
        .unwrap_or_else(generate_csrf_token);

    let mut response = Json(AuthSessionResponse {
        user: AuthUser {
            id: auth.user_id,
            username: auth.username,
            is_platform_admin: auth.is_platform_admin,
            status: auth.status,
            must_change_password: false,
        },
        auth_mode: "session".to_owned(),
    })
    .into_response();
    append_csrf_token(&mut response, &csrf, &state)?;
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Session revoked and cookie cleared", headers(("set-cookie" = String, description = "Cleared session cookie"))),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    revoke_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        clear_session_cookie_header(
            state.config().session_cookie_secure,
            state.config().session_cookie_same_site,
        )?,
    );
    response.headers_mut().append(
        header::SET_COOKIE,
        clear_csrf_cookie_header(
            state.config().session_cookie_secure,
            state.config().session_cookie_same_site,
        )?,
    );
    Ok(response)
}

fn append_csrf_token(
    response: &mut Response,
    csrf_token: &str,
    state: &AppState,
) -> Result<(), ApiError> {
    response.headers_mut().append(
        header::SET_COOKIE,
        csrf_cookie_header(
            csrf_token,
            state.config().session_cookie_secure,
            state.config().session_cookie_same_site,
        )?,
    );
    response
        .headers_mut()
        .insert(CSRF_HEADER_NAME, csrf_header_value(csrf_token)?);
    Ok(())
}

impl LoginRequest {
    fn validate(self) -> Result<ValidatedLoginRequest, ApiError> {
        Ok(ValidatedLoginRequest {
            username: required(self.username, "username")?,
            password: required(self.password, "password")?,
        })
    }
}

impl ChangePasswordRequest {
    fn validate(self) -> Result<ValidatedChangePasswordRequest, ApiError> {
        Ok(ValidatedChangePasswordRequest {
            current_password: required(self.current_password, "current_password")?,
            new_password: required(self.new_password, "new_password")?,
        })
    }
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = value else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    };

    if value.trim().is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    }

    Ok(value)
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "username" => "missing required body field: username",
        "password" => "missing required body field: password",
        "current_password" => "missing required body field: current_password",
        "new_password" => "missing required body field: new_password",
        _ => "missing required body field",
    }
}

fn require_login_csrf(headers: &HeaderMap) -> Result<(), ApiError> {
    let cookie_header = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let expected_token = csrf_token(cookie_header)
        .ok_or_else(|| ApiError::forbidden("csrf_token_missing", "Missing CSRF token cookie"))?;

    let actual_token = headers
        .get(CSRF_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::forbidden("csrf_token_missing", "Missing CSRF token header"))?;

    if actual_token != expected_token {
        return Err(ApiError::forbidden(
            "csrf_token_invalid",
            "CSRF token does not match",
        ));
    }

    Ok(())
}
