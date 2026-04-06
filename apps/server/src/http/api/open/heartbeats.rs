use crate::{
    auth::{authenticate_open_request, ensure_deployment_access},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::HeaderMap,
    routing::post,
};
use schema::open::DeploymentSyncResponse;
use serde::Deserialize;
use sqlx::{Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    project: Option<String>,
    environment: Option<String>,
    deployment_key: Option<String>,
    process_key: Option<String>,
    metadata: Option<serde_json::Value>,
    reported_at: Option<String>,
}

#[derive(Debug)]
struct ValidatedHeartbeatRequest {
    project: String,
    environment: String,
    deployment_key: String,
    process_key: String,
    metadata: Option<serde_json::Value>,
    reported_at: Option<String>,
}

#[derive(Debug)]
struct DeploymentLookup {
    project_id: i64,
    deployment_id: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/open/heartbeats", post(report_heartbeat))
}

async fn report_heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<HeartbeatRequest>, JsonRejection>,
) -> Result<Json<DeploymentSyncResponse>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_open_request(pool, &headers).await?;

    let deployment = find_deployment(
        pool,
        &payload.project,
        &payload.environment,
        &payload.deployment_key,
    )
    .await?
    .ok_or_else(|| {
        ApiError::not_found_with("deployment_not_found", "deployment instance not found")
    })?;
    ensure_deployment_access(auth, deployment.deployment_id)?;

    upsert_heartbeat(
        pool,
        deployment.project_id,
        deployment.deployment_id,
        payload,
    )
    .await?;

    Ok(Json(DeploymentSyncResponse { ok: true }))
}

impl HeartbeatRequest {
    fn validate(self) -> Result<ValidatedHeartbeatRequest, ApiError> {
        Ok(ValidatedHeartbeatRequest {
            project: required(self.project, "project")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            process_key: required(self.process_key, "process_key")?,
            metadata: self.metadata,
            reported_at: self.reported_at.filter(|value| !value.trim().is_empty()),
        })
    }
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = value else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    };

    if value.trim().is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    }

    Ok(value)
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "project" => "missing required body field: project",
        "environment" => "missing required body field: environment",
        "deployment_key" => "missing required body field: deployment_key",
        "process_key" => "missing required body field: process_key",
        _ => "missing required body field",
    }
}

async fn find_deployment(
    pool: &sqlx::PgPool,
    project: &str,
    environment: &str,
    deployment_key: &str,
) -> Result<Option<DeploymentLookup>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT p.id AS project_id, d.id AS deployment_id
        FROM projects p
        JOIN deployment_instances d ON d.project_id = p.id
        WHERE p.code = $1
          AND d.environment = $2
          AND d.deployment_key = $3
        "#,
    )
    .bind(project)
    .bind(environment)
    .bind(deployment_key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| DeploymentLookup {
        project_id: row.get("project_id"),
        deployment_id: row.get("deployment_id"),
    }))
}

async fn upsert_heartbeat(
    pool: &sqlx::PgPool,
    project_id: i64,
    deployment_id: i64,
    payload: ValidatedHeartbeatRequest,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO deployment_heartbeats (
            project_id,
            deployment_instance_id,
            process_key,
            metadata,
            reported_at
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            COALESCE($5::timestamptz, NOW())
        )
        ON CONFLICT (deployment_instance_id, process_key)
        DO UPDATE SET
            metadata = EXCLUDED.metadata,
            reported_at = EXCLUDED.reported_at,
            updated_at = NOW()
        "#,
    )
    .bind(project_id)
    .bind(deployment_id)
    .bind(payload.process_key)
    .bind(payload.metadata.map(SqlxJson))
    .bind(payload.reported_at)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(())
}
