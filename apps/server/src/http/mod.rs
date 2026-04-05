mod api;

use crate::{error::ApiError, state::AppState};
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .nest("/api", api::router())
        .fallback(|| async { ApiError::not_found() })
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
