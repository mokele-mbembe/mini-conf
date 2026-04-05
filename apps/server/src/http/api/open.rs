mod configs;
mod releases;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(configs::router())
        .merge(releases::router())
}
