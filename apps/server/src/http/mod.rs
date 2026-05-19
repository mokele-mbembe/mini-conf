pub(crate) mod api;

use crate::{
    auth::CSRF_HEADER_NAME,
    config::{AppConfig, AppEnv},
    error::ApiError,
    state::AppState,
};
use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderName, HeaderValue, Method, header},
    middleware::{self, Next},
    response::Response,
    routing::get_service,
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let static_dir = state.config().static_dir().to_path_buf();
    let api_router = apply_cors(api::router(state.clone()), state.config());

    let router = Router::new()
        .merge(crate::openapi::router())
        .nest("/api", api_router)
        .with_state(state.clone())
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
