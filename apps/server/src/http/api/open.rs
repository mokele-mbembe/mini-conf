mod configs;
mod deployments;
mod heartbeats;
mod releases;
mod sync_records;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(configs::router())
        .merge(deployments::router())
        .merge(heartbeats::router())
        .merge(releases::router())
        .merge(sync_records::router())
}
