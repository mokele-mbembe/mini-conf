pub(crate) mod admin_projects;
pub(crate) mod admin_users;
pub(crate) mod audit_logs;
pub(crate) mod auth;
pub(crate) mod clone_sources;
pub(crate) mod config_files;
pub(crate) mod deployment_heartbeats;
pub(crate) mod deployment_instances;
pub(crate) mod deployment_sync_records;
pub(crate) mod drafts;
pub(crate) mod health;
pub(crate) mod open;
pub(crate) mod project_environments;
pub(crate) mod project_members;
pub(crate) mod projects;
pub(crate) mod releases;
pub(crate) mod saved_versions;
pub(crate) mod setup;

use crate::{
    auth::{CSRF_HEADER_NAME, csrf_token},
    config::AppConfig,
    error::ApiError,
    state::AppState,
};
use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, Method, header},
    middleware::{self, Next},
    response::Response,
};
use sqlx::Row;

pub fn router(state: AppState) -> Router<AppState> {
    let setup_free_routes = Router::new()
        .merge(auth::router())
        .merge(health::router())
        .merge(setup::router())
        .merge(admin_projects::router())
        .merge(admin_users::router());

    let gated_session_routes = Router::new()
        .merge(audit_logs::router())
        .merge(clone_sources::router())
        .merge(config_files::router())
        .merge(deployment_instances::router())
        .merge(deployment_heartbeats::router())
        .merge(deployment_sync_records::router())
        .merge(drafts::router())
        .merge(project_members::router())
        .merge(project_environments::router())
        .merge(projects::router())
        .merge(releases::router())
        .merge(saved_versions::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_completed_setup,
        ));

    let gated_open_routes = open::router(state.clone()).route_layer(
        middleware::from_fn_with_state(state.clone(), require_completed_setup),
    );

    setup_free_routes
        .merge(gated_session_routes)
        .route_layer(middleware::from_fn_with_state(
            state,
            require_csrf_protection,
        ))
        .merge(gated_open_routes)
}

async fn require_completed_setup(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Ok(next.run(request).await);
    };

    let is_completed = sqlx::query(
        r#"
        SELECT setup_completed_at IS NOT NULL AS is_completed
        FROM system_settings
        WHERE id = 1
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to check setup completion"))?
    .map(|row| row.get::<bool, _>("is_completed"))
    .unwrap_or(false);

    if !is_completed {
        return Err(ApiError::conflict(
            "setup_required",
            "System setup is not complete",
        ));
    }

    Ok(next.run(request).await)
}

async fn require_csrf_protection(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    if matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    ) {
        return Ok(next.run(request).await);
    }

    require_allowed_origin(state.config(), request.headers())?;

    let cookie_header = request
        .headers()
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    let Some(expected_token) = csrf_token(cookie_header) else {
        return Ok(next.run(request).await);
    };

    let actual_token = request
        .headers()
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

    Ok(next.run(request).await)
}

fn require_allowed_origin(config: &AppConfig, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(origin) = header_value(headers, header::ORIGIN) else {
        return Ok(());
    };

    if config.is_cors_origin_allowed(origin) || is_same_origin(config, headers, origin) {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "origin_not_allowed",
        "Request origin is not allowed",
    ))
}

fn is_same_origin(config: &AppConfig, headers: &HeaderMap, origin: &str) -> bool {
    let Some(host) = first_header_segment(headers, "x-forwarded-host")
        .or_else(|| header_value(headers, header::HOST))
    else {
        return false;
    };
    let scheme = first_header_segment(headers, "x-forwarded-proto").unwrap_or(
        if config.session_cookie_secure {
            "https"
        } else {
            "http"
        },
    );
    let expected = format!("{scheme}://{host}");

    origin.eq_ignore_ascii_case(&expected)
}

fn first_header_segment<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    header_value(headers, name)?
        .split(',')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn header_value<K>(headers: &HeaderMap, name: K) -> Option<&str>
where
    K: axum::http::header::AsHeaderName,
{
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}
