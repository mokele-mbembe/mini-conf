pub(crate) mod admin_projects;
pub(crate) mod admin_users;
pub(crate) mod audit_logs;
pub(crate) mod auth;
pub(crate) mod clone_sources;
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
pub(crate) mod setup;

use crate::{error::ApiError, state::AppState};
use axum::{
    Router,
    extract::{Request, State},
    middleware::{self, Next},
    response::Response,
};
use sqlx::Row;

pub fn router(state: AppState) -> Router<AppState> {
    let setup_free_routes = Router::new()
        .merge(auth::router())
        .merge(health::router())
        .merge(setup::router());

    let gated_routes = Router::new()
        .merge(admin_projects::router())
        .merge(admin_users::router())
        .merge(audit_logs::router())
        .merge(clone_sources::router())
        .merge(config_files::router())
        .merge(deployment_instances::router())
        .merge(deployment_heartbeats::router())
        .merge(deployment_sync_records::router())
        .merge(drafts::router())
        .merge(open::router())
        .merge(project_members::router())
        .merge(project_environments::router())
        .merge(projects::router())
        .merge(releases::router())
        .merge(saved_versions::router())
        .route_layer(middleware::from_fn_with_state(
            state,
            require_completed_setup,
        ));

    setup_free_routes.merge(gated_routes)
}

async fn require_completed_setup(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Ok(next.run(request).await);
    };

    let is_completed = sqlx::query(
        r#"
        SELECT setup_completed_at IS NOT NULL AS is_completed
        FROM system_settings
        WHERE id = 1
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .map(|row| row.get::<bool, _>("is_completed"))
    .unwrap_or(false);

    if !is_completed {
        return Err(ApiError::conflict(
            "setup_required",
            "System setup is not complete",
        ));
    }

    Ok(next.run(request).await)
}
