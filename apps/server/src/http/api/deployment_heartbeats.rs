use crate::{
    authorization::{ProjectRole, authenticate_user, require_project_role},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use schema::audit::{DeploymentHeartbeatListResponse, DeploymentHeartbeatSummary};
use serde::Deserialize;
use sqlx::{Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
pub(crate) struct ListDeploymentHeartbeatsQuery {
    project_id: Option<i64>,
    deployment_instance_id: Option<i64>,
    process_key: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/deployment-heartbeats", get(list_deployment_heartbeats))
}

#[utoipa::path(
    get,
    path = "/api/deployment-heartbeats",
    tag = "admin",
    params(crate::openapi::ListDeploymentHeartbeatsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment heartbeats visible to the current project member", body = DeploymentHeartbeatListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_deployment_heartbeats(
    State(state): State<AppState>,
    Query(query): Query<ListDeploymentHeartbeatsQuery>,
    headers: HeaderMap,
) -> Result<Json<DeploymentHeartbeatListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_user(pool, &headers).await?;
    if let Some(project_id) = query.project_id {
        require_project_role(
            pool,
            auth.user_id,
            project_id,
            ProjectRole::Viewer,
            "project_not_found",
            "project not found",
        )
        .await?;
    }
    if let Some(deployment_instance_id) = query.deployment_instance_id {
        let project_id = load_deployment_project_id(pool, deployment_instance_id).await?;
        require_project_role(
            pool,
            auth.user_id,
            project_id,
            ProjectRole::Viewer,
            "deployment_instance_not_found",
            "deployment instance not found",
        )
        .await?;
    }

    let rows = sqlx::query(
        r#"
        SELECT
            dh.id,
            dh.project_id,
            dh.deployment_instance_id,
            dh.process_key,
            dh.metadata,
            to_char(dh.reported_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS reported_at
        FROM deployment_heartbeats dh
        JOIN project_members pm
          ON pm.project_id = dh.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR dh.project_id = $2)
          AND ($3::bigint IS NULL OR dh.deployment_instance_id = $3)
          AND ($4::varchar IS NULL OR dh.process_key = $4)
        ORDER BY dh.reported_at DESC, dh.process_key ASC, dh.id DESC
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(query.deployment_instance_id)
    .bind(normalize_optional(query.process_key))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(DeploymentHeartbeatListResponse {
        items: rows.iter().map(map_heartbeat_row).collect(),
    }))
}

fn map_heartbeat_row(row: &sqlx::postgres::PgRow) -> DeploymentHeartbeatSummary {
    DeploymentHeartbeatSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        deployment_instance_id: row.get("deployment_instance_id"),
        process_key: row.get("process_key"),
        metadata: row
            .get::<Option<SqlxJson<serde_json::Value>>, _>("metadata")
            .map(|value| value.0),
        reported_at: row.get("reported_at"),
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

async fn load_deployment_project_id(pool: &sqlx::PgPool, id: i64) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>(
        "SELECT project_id FROM deployment_instances WHERE id = $1 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })
}
