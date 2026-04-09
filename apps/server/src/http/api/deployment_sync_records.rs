use crate::{authorization::authenticate_user, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use schema::audit::{DeploymentSyncRecordListResponse, DeploymentSyncRecordSummary};
use serde::Deserialize;
use sqlx::{Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
pub(crate) struct ListDeploymentSyncRecordsQuery {
    project_id: Option<i64>,
    deployment_instance_id: Option<i64>,
    config_file_id: Option<i64>,
    process_key: Option<String>,
    action: Option<String>,
    status: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/deployment-sync-records",
        get(list_deployment_sync_records),
    )
}

#[utoipa::path(
    get,
    path = "/api/deployment-sync-records",
    tag = "admin",
    params(crate::openapi::ListDeploymentSyncRecordsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment sync records visible to the current project member", body = DeploymentSyncRecordListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_deployment_sync_records(
    State(state): State<AppState>,
    Query(query): Query<ListDeploymentSyncRecordsQuery>,
    headers: HeaderMap,
) -> Result<Json<DeploymentSyncRecordListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_user(pool, &headers).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            dsr.id,
            dsr.project_id,
            dsr.deployment_instance_id,
            dsr.config_file_id,
            dsr.release_id,
            dsr.process_key,
            dsr.revision,
            dsr.action,
            dsr.status,
            dsr.message,
            dsr.detail,
            to_char(dsr.reported_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS reported_at
        FROM deployment_sync_records dsr
        JOIN project_members pm
          ON pm.project_id = dsr.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR dsr.project_id = $2)
          AND ($3::bigint IS NULL OR dsr.deployment_instance_id = $3)
          AND ($4::bigint IS NULL OR dsr.config_file_id = $4)
          AND ($5::varchar IS NULL OR dsr.process_key = $5)
          AND ($6::varchar IS NULL OR dsr.action = $6)
          AND ($7::varchar IS NULL OR dsr.status = $7)
        ORDER BY dsr.reported_at DESC, dsr.id DESC
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(query.deployment_instance_id)
    .bind(query.config_file_id)
    .bind(normalize_optional(query.process_key))
    .bind(normalize_optional(query.action))
    .bind(normalize_optional(query.status))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(DeploymentSyncRecordListResponse {
        items: rows.iter().map(map_sync_record_row).collect(),
    }))
}

fn map_sync_record_row(row: &sqlx::postgres::PgRow) -> DeploymentSyncRecordSummary {
    DeploymentSyncRecordSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        deployment_instance_id: row.get("deployment_instance_id"),
        config_file_id: row.get("config_file_id"),
        release_id: row.get("release_id"),
        process_key: row.get("process_key"),
        revision: row.get("revision"),
        action: row.get("action"),
        status: row.get("status"),
        message: row.get("message"),
        detail: row
            .get::<Option<SqlxJson<serde_json::Value>>, _>("detail")
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
