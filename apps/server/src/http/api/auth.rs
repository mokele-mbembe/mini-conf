use crate::{
    auth::{
        authenticate_admin_session, clear_session_cookie_header, generate_session_token,
        hash_bearer_token, revoke_admin_session, session_cookie_header, verify_password,
    },
    error::ApiError,
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = crate::openapi::LoginRequestBody,
    responses(
        (status = 200, description = "Login successful and session cookie issued", body = AuthSessionResponse, headers(("set-cookie" = String, description = "HttpOnly session cookie"))),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Invalid username or password", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn login(
    State(state): State<AppState>,
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

    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash
        FROM users
        WHERE username = $1
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(&payload.username)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::unauthorized("invalid_credentials", "Invalid username or password"))?;

    let password_hash: String = row.get("password_hash");
    if !verify_password(&payload.password, &password_hash)? {
        return Err(ApiError::unauthorized(
            "invalid_credentials",
            "Invalid username or password",
        ));
    }

    let user_id: i64 = row.get("id");
    let username: String = row.get("username");
    let session_token = generate_session_token();

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
    .map_err(|_| ApiError::internal())?;

    let mut response = Json(AuthSessionResponse {
        user: AuthUser {
            id: user_id,
            username,
        },
        auth_mode: "session".to_owned(),
    })
    .into_response();
    response
        .headers_mut()
        .append(header::SET_COOKIE, session_cookie_header(&session_token));
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
) -> Result<Json<AuthSessionResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    Ok(Json(AuthSessionResponse {
        user: AuthUser {
            id: auth.user_id,
            username: auth.username,
        },
        auth_mode: "session".to_owned(),
    }))
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
    response
        .headers_mut()
        .append(header::SET_COOKIE, clear_session_cookie_header());
    Ok(response)
}

impl LoginRequest {
    fn validate(self) -> Result<ValidatedLoginRequest, ApiError> {
        Ok(ValidatedLoginRequest {
            username: required(self.username, "username")?,
            password: required(self.password, "password")?,
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
        _ => "missing required body field",
    }
}
