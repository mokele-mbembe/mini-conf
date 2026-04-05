mod health;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new().merge(health::router())
}
