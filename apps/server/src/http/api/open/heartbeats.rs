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
pub(crate) struct HeartbeatRequest {
    project: Option<String>,
    environment: Option<String>,
    deployment_key: Option<String>,
    config: Option<String>,
    metadata: Option<serde_json::Value>,
    reported_at: Option<String>,
}

#[derive(Debug)]
struct ValidatedHeartbeatRequest {
    project: String,
    environment: String,
    deployment_key: String,
    config: String,
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

#[utoipa::path(
    post,
    path = "/api/open/heartbeats",
    tag = "open",
    request_body = crate::openapi::HeartbeatRequestBody,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Heartbeat accepted", body = DeploymentSyncResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::error::ErrorResponse),
        (status = 403, description = "Bearer token does not match target deployment", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment not found", body = crate::error::ErrorResponse),
        (status = 429, description = "Open API rate limit exceeded", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn report_heartbeat(
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

    let config_file_id = find_config_file(pool, deployment.project_id, &payload.config)
        .await?
        .ok_or_else(|| {
            ApiError::not_found_with("config_file_not_found", "config file not found")
        })?;

    upsert_heartbeat(
        pool,
        deployment.project_id,
        deployment.deployment_id,
        config_file_id,
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
            config: required(self.config, "config")?,
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
        "config" => "missing required body field: config",
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
        JOIN project_environments pe
          ON pe.project_id = d.project_id
         AND pe.id = d.environment_id
        WHERE p.code = $1
          AND p.status = 'active'
          AND pe.code = $2
          AND d.deployment_key = $3
          AND d.status = 'active'
        "#,
    )
    .bind(project)
    .bind(environment)
    .bind(deployment_key)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to find deployment"))?;

    Ok(row.map(|row| DeploymentLookup {
        project_id: row.get("project_id"),
        deployment_id: row.get("deployment_id"),
    }))
}

async fn find_config_file(
    pool: &sqlx::PgPool,
    project_id: i64,
    config: &str,
) -> Result<Option<i64>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM config_files
        WHERE project_id = $1
          AND code = $2
          AND status = 'active'
        "#,
    )
    .bind(project_id)
    .bind(config)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to find config file"))?;

    Ok(row.map(|row| row.get("id")))
}

async fn upsert_heartbeat(
    pool: &sqlx::PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
    payload: ValidatedHeartbeatRequest,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO deployment_heartbeats (
            project_id,
            deployment_instance_id,
            config_file_id,
            metadata,
            reported_at
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            COALESCE($5::timestamptz, NOW())
        )
        ON CONFLICT (deployment_instance_id, config_file_id)
        DO UPDATE SET
            metadata = EXCLUDED.metadata,
            reported_at = EXCLUDED.reported_at,
            updated_at = NOW()
        "#,
    )
    .bind(project_id)
    .bind(deployment_id)
    .bind(config_file_id)
    .bind(payload.metadata.map(SqlxJson))
    .bind(payload.reported_at)
    .execute(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to upsert heartbeat"))?;

    Ok(())
}
