pub(crate) mod api;

use crate::{error::ApiError, state::AppState};
use axum::{Router, routing::get_service};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let static_dir = state.config().static_dir().to_path_buf();

    let router = Router::new()
        .merge(crate::openapi::router())
        .nest("/api", api::router(state.clone()))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

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
