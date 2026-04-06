mod auth;
pub(crate) mod health;
pub(crate) mod open;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(auth::router())
        .merge(health::router())
        .merge(open::router())
}
