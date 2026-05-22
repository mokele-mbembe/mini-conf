pub(crate) mod api;

use crate::{
    auth::CSRF_HEADER_NAME,
    config::{AppConfig, AppEnv},
    error::ApiError,
    state::AppState,
};
use axum::{
    Router,
    extract::{MatchedPath, Request, State},
    http::{HeaderName, HeaderValue, Method, header},
    middleware::{self, Next},
    response::Response,
    routing::get_service,
};
use std::time::Instant;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let static_dir = state.config().static_dir().to_path_buf();
    let api_router = apply_cors(api::router(state.clone()), state.config());

    let router = Router::new()
        .merge(crate::metrics::router())
        .merge(crate::openapi::router())
        .nest("/api", api_router)
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            record_http_metrics,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state, add_security_headers));

    if static_dir.is_dir() {
        let static_service = get_service(
            ServeDir::new(&static_dir)
                .append_index_html_on_directories(true)
                .not_found_service(ServeFile::new(static_dir.join("index.html"))),
        );

        router.fallback_service(static_service)
    } else {
        router.fallback(|| async { ApiError::not_found() })
    }
}

async fn record_http_metrics(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().as_str().to_owned();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched_path| matched_path.as_str().to_owned())
        .unwrap_or_else(|| route_label(request.uri().path()));
    let started_at = Instant::now();

    let response = next.run(request).await;

    state
        .metrics()
        .record_http_request(&method, &route, response.status(), started_at.elapsed());

    response
}

async fn add_security_headers(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let include_hsts = matches!(state.config().app_env, AppEnv::Staging | AppEnv::Prod);

    crate::security::apply_security_headers(
        response.headers_mut(),
        include_hsts,
        state.config().csp_connect_src_extra(),
    );
    response
}

fn apply_cors(router: Router<AppState>, config: &AppConfig) -> Router<AppState> {
    let origins: Vec<HeaderValue> = config
        .cors_allowed_origins()
        .iter()
        .filter_map(|origin| match HeaderValue::from_str(origin) {
            Ok(origin) => Some(origin),
            Err(error) => {
                tracing::error!(
                    ?error,
                    origin,
                    "configured CORS origin is not a valid header"
                );
                None
            }
        })
        .collect();

    if origins.is_empty() {
        return router;
    }

    router.layer(
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::ACCEPT,
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::IF_NONE_MATCH,
                HeaderName::from_static(CSRF_HEADER_NAME),
            ])
            .expose_headers([header::ETAG, HeaderName::from_static(CSRF_HEADER_NAME)]),
    )
}

fn route_label(path: &str) -> String {
    let normalized = path
        .split('/')
        .map(|segment| {
            if segment.is_empty() {
                ""
            } else if segment.chars().all(|ch| ch.is_ascii_digit()) {
                "{id}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    if normalized.is_empty() {
        "/".to_owned()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::route_label;

    #[test]
    fn route_label_normalizes_numeric_segments() {
        assert_eq!(
            route_label("/api/projects/42/deployments/7/preview"),
            "/api/projects/{id}/deployments/{id}/preview"
        );
        assert_eq!(route_label("/metrics"), "/metrics");
    }
}
