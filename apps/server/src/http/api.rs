pub(crate) mod audit_logs;
pub(crate) mod auth;
pub(crate) mod config_files;
pub(crate) mod deployment_heartbeats;
pub(crate) mod deployment_instances;
pub(crate) mod deployment_sync_records;
pub(crate) mod drafts;
pub(crate) mod health;
pub(crate) mod open;
pub(crate) mod project_environments;
pub(crate) mod project_members;
pub(crate) mod projects;
pub(crate) mod releases;
pub(crate) mod saved_versions;

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(audit_logs::router())
        .merge(auth::router())
        .merge(config_files::router())
        .merge(deployment_instances::router())
        .merge(deployment_heartbeats::router())
        .merge(deployment_sync_records::router())
        .merge(drafts::router())
        .merge(health::router())
        .merge(open::router())
        .merge(project_members::router())
        .merge(project_environments::router())
        .merge(projects::router())
        .merge(releases::router())
        .merge(saved_versions::router())
}
