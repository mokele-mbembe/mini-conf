pub(crate) mod api;

use crate::{config::AppEnv, error::ApiError, state::AppState};
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
    routing::get_service,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let static_dir = state.config().static_dir().to_path_buf();

    let router = Router::new()
        .merge(crate::openapi::router())
        .nest("/api", api::router(state.clone()))
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

    crate::security::apply_security_headers(response.headers_mut(), include_hsts);
    response
}
