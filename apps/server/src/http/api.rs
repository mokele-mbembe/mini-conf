pub(crate) mod auth;
pub(crate) mod config_files;
pub(crate) mod deployment_instances;
pub(crate) mod drafts;
pub(crate) mod health;
pub(crate) mod open;
pub(crate) mod projects;
pub(crate) mod releases;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(auth::router())
        .merge(config_files::router())
        .merge(deployment_instances::router())
        .merge(drafts::router())
        .merge(health::router())
        .merge(open::router())
        .merge(projects::router())
        .merge(releases::router())
}
