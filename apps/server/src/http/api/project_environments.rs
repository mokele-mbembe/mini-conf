use crate::{
    audit::{AuditLogEntry, write_audit_log},
    authorization::{ProjectRole, authenticate_user, require_project_role},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use schema::project_environment::{ProjectEnvironmentListResponse, ProjectEnvironmentSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateProjectEnvironmentRequest {
    code: Option<String>,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Debug)]
struct ValidatedCreateProjectEnvironmentRequest {
    code: String,
    name: String,
    description: Option<String>,
    status: String,
    sort_order: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateProjectEnvironmentRequest {
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
    sort_order: Option<i32>,
}

#[derive(Debug)]
struct ValidatedUpdateProjectEnvironmentRequest {
    name: String,
    description: Option<String>,
    status: String,
    sort_order: i32,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{project_id}/environments",
            get(list_project_environments).post(create_project_environment),
        )
        .route(
            "/projects/{project_id}/environments/{environment_id}",
            get(get_project_environment)
                .put(update_project_environment)
                .delete(delete_project_environment),
        )
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/environments",
    tag = "admin",
    params(
        ("project_id" = i64, Path, description = "Project ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List project environments", body = ProjectEnvironmentListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_project_environments(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ProjectEnvironmentListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Viewer,
        "project_not_found",
        "project not found",
    )
    .await?;

    let rows = sqlx::query(
        r#"
        SELECT
            pe.id,
            pe.project_id,
            pe.code,
            pe.name,
            pe.description,
            pe.status,
            pe.sort_order,
            COUNT(di.id)::bigint AS deployment_count
        FROM project_environments pe
        LEFT JOIN deployment_instances di
          ON di.project_id = pe.project_id
         AND di.environment_id = pe.id
        WHERE pe.project_id = $1
        GROUP BY pe.id, pe.project_id, pe.code, pe.name, pe.description, pe.status, pe.sort_order
        ORDER BY pe.sort_order ASC, pe.code ASC, pe.id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(ProjectEnvironmentListResponse {
        items: rows.into_iter().map(map_project_environment_row).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/projects/{project_id}/environments/{environment_id}",
    tag = "admin",
    params(
        ("project_id" = i64, Path, description = "Project ID"),
        ("environment_id" = i64, Path, description = "Project environment ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project environment detail", body = ProjectEnvironmentSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or environment not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_project_environment(
    State(state): State<AppState>,
    Path((project_id, environment_id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Json<ProjectEnvironmentSummary>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Viewer,
        "project_not_found",
        "project not found",
    )
    .await?;

    let row = sqlx::query(
        r#"
        SELECT
            pe.id,
            pe.project_id,
            pe.code,
            pe.name,
            pe.description,
            pe.status,
            pe.sort_order,
            COUNT(di.id)::bigint AS deployment_count
        FROM project_environments pe
        LEFT JOIN deployment_instances di
          ON di.project_id = pe.project_id
         AND di.environment_id = pe.id
        WHERE pe.project_id = $1
          AND pe.id = $2
        GROUP BY pe.id, pe.project_id, pe.code, pe.name, pe.description, pe.status, pe.sort_order
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "project_environment_not_found",
            "project environment not found",
        )
    })?;

    Ok(Json(map_project_environment_row(row)))
}

#[utoipa::path(
    post,
    path = "/api/projects/{project_id}/environments",
    tag = "admin",
    params(
        ("project_id" = i64, Path, description = "Project ID")
    ),
    request_body = crate::openapi::CreateProjectEnvironmentRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Project environment created", body = ProjectEnvironmentSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current project member cannot update the project", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project environment code already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_project_environment(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<CreateProjectEnvironmentRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProjectEnvironmentSummary>), ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        INSERT INTO project_environments (
            project_id,
            code,
            name,
            description,
            status,
            sort_order
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id,
            project_id,
            code,
            name,
            description,
            status,
            sort_order,
            0::bigint AS deployment_count
        "#,
    )
    .bind(project_id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.sort_order)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_project_environment_write_error)?;

    let summary = map_project_environment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_environment.created",
            resource_type: "project_environment",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "project_environment_id": summary.id,
                "code": summary.code,
                "name": summary.name,
                "status": summary.status,
                "sort_order": summary.sort_order
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(summary)))
}

#[utoipa::path(
    put,
    path = "/api/projects/{project_id}/environments/{environment_id}",
    tag = "admin",
    params(
        ("project_id" = i64, Path, description = "Project ID"),
        ("environment_id" = i64, Path, description = "Project environment ID")
    ),
    request_body = crate::openapi::UpdateProjectEnvironmentRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project environment updated", body = ProjectEnvironmentSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current project member cannot update the project", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or environment not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_project_environment(
    State(state): State<AppState>,
    Path((project_id, environment_id)): Path<(i64, i64)>,
    headers: HeaderMap,
    payload: Result<Json<UpdateProjectEnvironmentRequest>, JsonRejection>,
) -> Result<Json<ProjectEnvironmentSummary>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        UPDATE project_environments
        SET
            name = $3,
            description = $4,
            status = $5,
            sort_order = $6,
            updated_at = NOW()
        WHERE project_id = $1
          AND id = $2
        RETURNING
            id,
            project_id,
            code,
            name,
            description,
            status,
            sort_order,
            (
                SELECT COUNT(*)
                FROM deployment_instances di
                WHERE di.project_id = project_environments.project_id
                  AND di.environment_id = project_environments.id
            )::bigint AS deployment_count
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .bind(payload.sort_order)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_project_environment_write_error)?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "project_environment_not_found",
            "project environment not found",
        )
    })?;

    let summary = map_project_environment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_environment.updated",
            resource_type: "project_environment",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "project_environment_id": summary.id,
                "changed_fields": ["name", "description", "status", "sort_order"]
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(summary))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{project_id}/environments/{environment_id}",
    tag = "admin",
    params(
        ("project_id" = i64, Path, description = "Project ID"),
        ("environment_id" = i64, Path, description = "Project environment ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Project environment deleted"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current project member cannot update the project", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or environment not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project environment is in use", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_project_environment(
    State(state): State<AppState>,
    Path((project_id, environment_id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    let exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM project_environments
        WHERE project_id = $1
          AND id = $2
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    if exists == 0 {
        return Err(ApiError::not_found_with(
            "project_environment_not_found",
            "project environment not found",
        ));
    }

    let deployment_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM deployment_instances
        WHERE project_id = $1
          AND environment_id = $2
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    if deployment_count > 0 {
        return Err(ApiError::conflict(
            "project_environment_in_use",
            "project environment is in use by deployment instances",
        ));
    }

    sqlx::query(
        r#"
        DELETE FROM project_environments
        WHERE project_id = $1
          AND id = $2
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_environment.deleted",
            resource_type: "project_environment",
            resource_id: environment_id.to_string(),
            detail: Some(serde_json::json!({
                "project_environment_id": environment_id,
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(StatusCode::NO_CONTENT)
}

impl CreateProjectEnvironmentRequest {
    fn validate(self) -> Result<ValidatedCreateProjectEnvironmentRequest, ApiError> {
        Ok(ValidatedCreateProjectEnvironmentRequest {
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            status: validate_status(self.status, "active")?,
            sort_order: self.sort_order.unwrap_or(0),
        })
    }
}

impl UpdateProjectEnvironmentRequest {
    fn validate(self) -> Result<ValidatedUpdateProjectEnvironmentRequest, ApiError> {
        Ok(ValidatedUpdateProjectEnvironmentRequest {
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            status: validate_status(self.status, "active")?,
            sort_order: self.sort_order.unwrap_or(0),
        })
    }
}

fn map_project_environment_row(row: sqlx::postgres::PgRow) -> ProjectEnvironmentSummary {
    ProjectEnvironmentSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        status: row.get("status"),
        sort_order: row.get("sort_order"),
        deployment_count: row.get("deployment_count"),
    }
}

fn map_project_environment_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error
        && database_error.constraint() == Some("project_environments_project_id_code_key")
    {
        return ApiError::conflict(
            "project_environment_code_conflict",
            "project environment code already exists",
        );
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
        "code" => "missing required body field: code",
        "name" => "missing required body field: name",
        "status" => "missing required body field: status",
        _ => "missing required body field",
    }
}

fn validate_status(value: Option<String>, default_status: &str) -> Result<String, ApiError> {
    let value = normalize_optional(value).unwrap_or_else(|| default_status.to_owned());

    match value.as_str() {
        "active" | "inactive" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid project environment status",
        )),
    }
}
