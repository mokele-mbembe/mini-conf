use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    routing::get,
};
use schema::deployment_instance::{DeploymentInstanceListResponse, DeploymentInstanceSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct ListDeploymentInstancesQuery {
    project_id: Option<i64>,
    environment: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateDeploymentInstanceRequest {
    project_id: Option<i64>,
    environment: Option<String>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    is_template: Option<bool>,
}

#[derive(Debug)]
struct ValidatedCreateDeploymentInstanceRequest {
    project_id: i64,
    environment: String,
    deployment_key: String,
    name: String,
    description: Option<String>,
    is_template: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateDeploymentInstanceRequest {
    project_id: Option<i64>,
    environment: Option<String>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    is_template: Option<bool>,
    status: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateDeploymentInstanceRequest {
    project_id: i64,
    environment: String,
    deployment_key: String,
    name: String,
    description: Option<String>,
    is_template: bool,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/deployment-instances",
            get(list_deployment_instances).post(create_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}",
            get(get_deployment_instance).put(update_deployment_instance),
        )
}

#[utoipa::path(
    get,
    path = "/api/deployment-instances",
    tag = "admin",
    params(crate::openapi::ListDeploymentInstancesParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List deployment instances", body = DeploymentInstanceListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_deployment_instances(
    State(state): State<AppState>,
    Query(query): Query<ListDeploymentInstancesQuery>,
    headers: HeaderMap,
) -> Result<Json<DeploymentInstanceListResponse>, ApiError> {
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
        SELECT
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        FROM deployment_instances
        WHERE ($1::bigint IS NULL OR project_id = $1)
          AND ($2::varchar IS NULL OR environment = $2)
          AND ($3::varchar IS NULL OR status = $3)
        ORDER BY project_id ASC, environment ASC, deployment_key ASC, id ASC
        "#,
    )
    .bind(query.project_id)
    .bind(normalize_optional(query.environment))
    .bind(normalize_optional(query.status))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(DeploymentInstanceListResponse {
        items: rows.into_iter().map(map_deployment_row).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances",
    tag = "admin",
    request_body = crate::openapi::CreateDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Deployment instance created", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment key already exists within the project/environment", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_deployment_instance(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateDeploymentInstanceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DeploymentInstanceSummary>), ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

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

    let row = sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            status
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.environment)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.is_template)
    .fetch_one(pool)
    .await
    .map_err(map_deployment_write_error)?;

    Ok((StatusCode::CREATED, Json(map_deployment_row(row))))
}

#[utoipa::path(
    get,
    path = "/api/deployment-instances/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance detail", body = DeploymentInstanceSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<DeploymentInstanceSummary>, ApiError> {
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

    let row = sqlx::query(
        r#"
        SELECT
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
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
    })?;

    Ok(Json(map_deployment_row(row)))
}

#[utoipa::path(
    put,
    path = "/api/deployment-instances/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    request_body = crate::openapi::UpdateDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance updated", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment key already exists within the project/environment", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateDeploymentInstanceRequest>, JsonRejection>,
) -> Result<Json<DeploymentInstanceSummary>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

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

    let row = sqlx::query(
        r#"
        UPDATE deployment_instances
        SET
            project_id = $2,
            environment = $3,
            deployment_key = $4,
            name = $5,
            description = $6,
            is_template = $7,
            status = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        "#,
    )
    .bind(id)
    .bind(payload.project_id)
    .bind(payload.environment)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.is_template)
    .bind(payload.status)
    .fetch_optional(pool)
    .await
    .map_err(map_deployment_write_error)?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(Json(map_deployment_row(row)))
}

impl CreateDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedCreateDeploymentInstanceRequest, ApiError> {
        Ok(ValidatedCreateDeploymentInstanceRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            is_template: self.is_template.unwrap_or(false),
        })
    }
}

impl UpdateDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedUpdateDeploymentInstanceRequest, ApiError> {
        Ok(ValidatedUpdateDeploymentInstanceRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            is_template: self.is_template.unwrap_or(false),
            status: validate_status(self.status)?,
        })
    }
}

fn map_deployment_row(row: sqlx::postgres::PgRow) -> DeploymentInstanceSummary {
    DeploymentInstanceSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        environment: row.get("environment"),
        deployment_key: row.get("deployment_key"),
        name: row.get("name"),
        description: row.get("description"),
        is_template: row.get("is_template"),
        template_source_id: row.get("template_source_id"),
        status: row.get("status"),
    }
}

fn map_deployment_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error {
        if database_error.constraint()
            == Some("deployment_instances_project_id_environment_deployment_key_key")
        {
            return ApiError::conflict(
                "deployment_key_conflict",
                "deployment key already exists in project environment",
            );
        }

        if database_error.constraint() == Some("deployment_instances_project_id_fkey") {
            return ApiError::not_found_with("project_not_found", "project not found");
        }
    }

    ApiError::internal()
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    };

    Ok(value)
}

fn required_i64(value: Option<i64>, field: &'static str) -> Result<i64, ApiError> {
    value.ok_or_else(|| ApiError::bad_request("invalid_request", invalid_body_message(field)))
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

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "project_id" => "missing required body field: project_id",
        "environment" => "missing required body field: environment",
        "deployment_key" => "missing required body field: deployment_key",
        "name" => "missing required body field: name",
        "status" => "missing required body field: status",
        _ => "missing required body field",
    }
}

fn validate_status(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message("status"),
        ));
    };

    match value.as_str() {
        "active" | "archived" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid deployment instance status",
        )),
    }
}
