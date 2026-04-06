use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    routing::get,
};
use schema::project::{ProjectListResponse, ProjectSummary};
use sqlx::Row;

pub fn router() -> Router<AppState> {
    Router::new().route("/projects", get(list_projects))
}

#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "admin",
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List active projects visible to the current admin session", body = ProjectListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    let rows = sqlx::query(
        r#"
        SELECT id, code, name, description, status
        FROM projects
        WHERE status = 'active'
        ORDER BY code ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let items = rows
        .into_iter()
        .map(|row| ProjectSummary {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
        })
        .collect();

    Ok(Json(ProjectListResponse { items }))
}
