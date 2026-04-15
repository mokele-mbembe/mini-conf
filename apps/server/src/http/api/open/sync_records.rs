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
pub(crate) struct DeploymentSyncRecordRequest {
    project: Option<String>,
    environment: Option<String>,
    deployment_key: Option<String>,
    config: Option<String>,
    action: Option<String>,
    revision: Option<String>,
    status: Option<String>,
    message: Option<String>,
    detail: Option<serde_json::Value>,
    reported_at: Option<String>,
}

#[derive(Debug)]
struct ValidatedDeploymentSyncRecordRequest {
    project: String,
    environment: String,
    deployment_key: String,
    config: String,
    action: String,
    revision: Option<String>,
    status: String,
    message: Option<String>,
    detail: Option<serde_json::Value>,
    reported_at: Option<String>,
}

#[derive(Debug)]
struct DeploymentLookup {
    project_id: i64,
    deployment_id: i64,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/open/deployment-sync-records", post(create_sync_record))
}

#[utoipa::path(
    post,
    path = "/api/open/deployment-sync-records",
    tag = "open",
    request_body = crate::openapi::DeploymentSyncRecordRequestBody,
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Sync record accepted", body = DeploymentSyncResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::error::ErrorResponse),
        (status = 403, description = "Bearer token does not match target deployment", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment, config file, or release not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_sync_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<DeploymentSyncRecordRequest>, JsonRejection>,
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

    let release_id = match payload.revision.as_deref() {
        Some(revision) => Some(
            find_release_id(pool, deployment.deployment_id, config_file_id, revision)
                .await?
                .ok_or_else(|| {
                    ApiError::not_found_with("release_not_found", "release not found")
                })?,
        ),
        None => None,
    };

    insert_sync_record(
        pool,
        deployment.project_id,
        deployment.deployment_id,
        config_file_id,
        release_id,
        payload,
    )
    .await?;

    Ok(Json(DeploymentSyncResponse { ok: true }))
}

impl DeploymentSyncRecordRequest {
    fn validate(self) -> Result<ValidatedDeploymentSyncRecordRequest, ApiError> {
        let action = required(self.action, "action")?;
        validate_action(&action)?;

        let status = required(self.status, "status")?;
        validate_status(&status)?;

        Ok(ValidatedDeploymentSyncRecordRequest {
            project: required(self.project, "project")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            config: required(self.config, "config")?,
            action,
            revision: self.revision.filter(|value| !value.trim().is_empty()),
            status,
            message: self.message.filter(|value| !value.trim().is_empty()),
            detail: self.detail,
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
        "action" => "missing required body field: action",
        "status" => "missing required body field: status",
        _ => "missing required body field",
    }
}

fn validate_action(action: &str) -> Result<(), ApiError> {
    match action {
        "version_check" | "fetch" | "apply" | "heartbeat" => Ok(()),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "unsupported action value",
        )),
    }
}

fn validate_status(status: &str) -> Result<(), ApiError> {
    match status {
        "success" | "noop" | "failed" => Ok(()),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "unsupported status value",
        )),
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
    .map_err(|_| ApiError::internal())?;

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
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| row.get("id")))
}

async fn find_release_id(
    pool: &sqlx::PgPool,
    deployment_id: i64,
    config_file_id: i64,
    revision: &str,
) -> Result<Option<i64>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM releases
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
          AND revision = $3
        "#,
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .bind(revision)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| row.get("id")))
}

async fn insert_sync_record(
    pool: &sqlx::PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
    release_id: Option<i64>,
    payload: ValidatedDeploymentSyncRecordRequest,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO deployment_sync_records (
            project_id,
            deployment_instance_id,
            config_file_id,
            release_id,
            revision,
            action,
            status,
            message,
            detail,
            reported_at
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            COALESCE($10::timestamptz, NOW())
        )
        "#,
    )
    .bind(project_id)
    .bind(deployment_id)
    .bind(config_file_id)
    .bind(release_id)
    .bind(payload.revision)
    .bind(payload.action)
    .bind(payload.status)
    .bind(payload.message)
    .bind(payload.detail.map(SqlxJson))
    .bind(payload.reported_at)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(())
}
