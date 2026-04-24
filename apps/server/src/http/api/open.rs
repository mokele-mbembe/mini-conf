pub(crate) mod configs;
pub(crate) mod deployments;
pub(crate) mod heartbeats;
pub(crate) mod releases;
pub(crate) mod sync_records;

use crate::{
    audit::{AuditLogEntry, write_audit_log_best_effort},
    error::ApiError,
    security::{
        OPEN_API_RATE_LIMIT, OPEN_API_RATE_WINDOW_SECS, has_bearer_token, open_api_rate_limit_keys,
        request_client_ip,
    },
    state::AppState,
};
use axum::{
    Router,
    extract::{OriginalUri, Request, State},
    http::{Method, StatusCode},
    middleware::{self, Next},
    response::Response,
};

#[derive(Debug)]
struct OpenApiFailureContext {
    method: Method,
    path: String,
    client_ip: String,
    has_bearer_token: bool,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(configs::router())
        .merge(deployments::router())
        .merge(heartbeats::router())
        .merge(releases::router())
        .merge(sync_records::router())
        .route_layer(middleware::from_fn_with_state(
            state,
            enforce_open_api_baseline,
        ))
}

async fn enforce_open_api_baseline(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let context = OpenApiFailureContext {
        method: request.method().clone(),
        path: request_path(&request),
        client_ip: request_client_ip(request.headers()),
        has_bearer_token: has_bearer_token(request.headers()),
    };
    let rate_limit_keys = open_api_rate_limit_keys(request.headers());

    if let Err(error) = state
        .open_api_rate_limiter()
        .ensure_request_allowed_for_keys(&rate_limit_keys)
    {
        write_open_api_failure(
            &state,
            &context,
            StatusCode::TOO_MANY_REQUESTS,
            Some("open_api_rate_limited"),
        )
        .await;
        return Err(error);
    }

    let response = next.run(request).await;
    let status = response.status();

    if should_audit_open_api_failure(status) {
        write_open_api_failure(&state, &context, status, None).await;
    }

    Ok(response)
}

fn request_path(request: &Request) -> String {
    request
        .extensions()
        .get::<OriginalUri>()
        .map(|uri| uri.0.path().to_owned())
        .unwrap_or_else(|| request.uri().path().to_owned())
}

fn should_audit_open_api_failure(status: StatusCode) -> bool {
    status.is_client_error() || status.is_server_error()
}

async fn write_open_api_failure(
    state: &AppState,
    context: &OpenApiFailureContext,
    status: StatusCode,
    error_code: Option<&'static str>,
) {
    let Some(pool) = state.db_pool() else {
        return;
    };

    let detail = serde_json::json!({
        "method": context.method.as_str(),
        "path": &context.path,
        "status": status.as_u16(),
        "client_ip": &context.client_ip,
        "has_bearer_token": context.has_bearer_token,
        "error_code": error_code,
        "rate_limit": {
            "max_requests": OPEN_API_RATE_LIMIT,
            "window_seconds": OPEN_API_RATE_WINDOW_SECS,
        },
    });

    write_audit_log_best_effort(
        pool,
        AuditLogEntry {
            project_id: None,
            user_id: None,
            action: "open_api.request_failed",
            resource_type: "open_api",
            resource_id: audit_resource_id(&context.path),
            detail: Some(detail),
        },
    )
    .await;
}

fn audit_resource_id(path: &str) -> String {
    let resource_id: String = path.chars().take(64).collect();

    if resource_id.is_empty() {
        "open_api".to_owned()
    } else {
        resource_id
    }
}

#[cfg(test)]
mod tests {
    use super::{audit_resource_id, should_audit_open_api_failure};
    use axum::http::StatusCode;

    #[test]
    fn audit_decision_records_client_and_server_errors_only() {
        assert!(!should_audit_open_api_failure(StatusCode::OK));
        assert!(!should_audit_open_api_failure(StatusCode::NOT_MODIFIED));
        assert!(should_audit_open_api_failure(StatusCode::BAD_REQUEST));
        assert!(should_audit_open_api_failure(StatusCode::UNAUTHORIZED));
        assert!(should_audit_open_api_failure(StatusCode::TOO_MANY_REQUESTS));
        assert!(should_audit_open_api_failure(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
    }

    #[test]
    fn audit_resource_id_defaults_and_truncates_to_column_limit() {
        assert_eq!(audit_resource_id(""), "open_api");
        assert_eq!(
            audit_resource_id("/api/open/configs/resolve"),
            "/api/open/configs/resolve"
        );

        let long_path = format!("/api/open/{}", "x".repeat(100));
        let resource_id = audit_resource_id(&long_path);

        assert_eq!(resource_id.chars().count(), 64);
        assert!(long_path.starts_with(&resource_id));
    }
}
