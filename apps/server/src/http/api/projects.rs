use crate::{
    audit::{AuditLogEntry, write_audit_log},
    authorization::{ProjectRole, authenticate_user, require_platform_admin, require_project_role},
    error::ApiError,
    http::api::admin_projects::create_platform_project_with_initial_admin,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::HeaderMap,
    http::StatusCode,
    routing::get,
};
use schema::admin_project::CreatePlatformProjectResponse;
use schema::project::{ProjectListResponse, ProjectSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct ListProjectsQuery {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateProjectRequest {
    code: Option<String>,
    name: Option<String>,
    description: Option<String>,
    initial_admin_user_id: Option<i64>,
}

#[derive(Debug)]
struct ValidatedCreateProjectRequest {
    code: String,
    name: String,
    description: Option<String>,
    initial_admin_user_id: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateProjectRequest {
    code: Option<String>,
    name: Option<String>,
    description: Option<String>,
    status: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateProjectRequest {
    code: String,
    name: String,
    description: Option<String>,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
}

#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "admin",
    params(crate::openapi::ListProjectsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List active projects visible to the current session user", body = ProjectListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_projects(
    State(state): State<AppState>,
    Query(query): Query<ListProjectsQuery>,
    headers: HeaderMap,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_user(pool, &headers).await?;

    let rows = sqlx::query(
        r#"
        SELECT DISTINCT p.id, p.code, p.name, p.description, p.status, pm.role AS current_user_role
        FROM projects p
        JOIN project_members pm
          ON pm.project_id = p.id
         AND pm.user_id = $1
        WHERE (
                $2::varchar IS NULL
                AND p.status IN ('active', 'archived')
              )
           OR (
                $2::varchar IS NOT NULL
                AND p.status = $2
              )
        ORDER BY p.code ASC, p.id ASC
        "#,
    )
    .bind(auth.user_id)
    .bind(normalize_optional(query.status))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let items = rows.iter().map(map_project_row).collect();

    Ok(Json(ProjectListResponse { items }))
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project detail", body = ProjectSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ProjectSummary>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_user(pool, &headers).await?;

    let row = sqlx::query(
        r#"
        SELECT p.id, p.code, p.name, p.description, p.status, pm.role AS current_user_role
        FROM projects p
        JOIN project_members pm
          ON pm.project_id = p.id
         AND pm.user_id = $2
        WHERE p.id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("project_not_found", "project not found"))?;

    Ok(Json(map_project_row(&row)))
}

#[utoipa::path(
    put,
    path = "/api/projects/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    request_body = crate::openapi::UpdateProjectRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project updated", body = ProjectSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current project member cannot update the project", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project code already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateProjectRequest>, JsonRejection>,
) -> Result<Json<ProjectSummary>, ApiError> {
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
        id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        UPDATE projects
        SET
            code = $2,
            name = $3,
            description = $4,
            status = $5,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, code, name, description, status, 'admin'::varchar AS current_user_role
        "#,
    )
    .bind(id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.status)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_project_write_error)?
    .ok_or_else(|| ApiError::not_found_with("project_not_found", "project not found"))?;

    let summary = map_project_row(&row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(id),
            user_id: Some(auth.user_id),
            action: "project.updated",
            resource_type: "project",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "changed_fields": ["code", "name", "description", "status"]
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(summary))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Project deleted"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current project member cannot delete the project", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project has dependent resources", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
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
        id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    delete_project_by_id(pool, auth.user_id, id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "admin",
    request_body = crate::openapi::CreatePlatformProjectRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Deprecated alias for platform project creation", body = CreatePlatformProjectResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "Initial admin user not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project code already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateProjectRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreatePlatformProjectResponse>), ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = require_platform_admin(pool, &headers).await?;

    let response = create_platform_project_with_initial_admin(
        pool,
        auth.user_id,
        payload.code,
        payload.name,
        payload.description,
        payload.initial_admin_user_id,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(response)))
}

pub(crate) async fn delete_project_by_id(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    project_id: i64,
) -> Result<(), ApiError> {
    let project_row = sqlx::query(
        r#"
        SELECT id, code, name, status
        FROM projects
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("project_not_found", "project not found"))?;

    let reference_row = sqlx::query(
        r#"
        SELECT
            (SELECT COUNT(*) FROM config_files WHERE project_id = $1)::bigint AS config_file_count,
            (SELECT COUNT(*) FROM deployment_instances WHERE project_id = $1)::bigint AS deployment_count,
            (SELECT COUNT(*) FROM project_environments WHERE project_id = $1)::bigint AS environment_count,
            (SELECT COUNT(*) FROM releases WHERE project_id = $1)::bigint AS release_count,
            (SELECT COUNT(*) FROM drafts WHERE project_id = $1)::bigint AS draft_count,
            (SELECT COUNT(*) FROM draft_saved_versions WHERE project_id = $1)::bigint AS saved_version_count,
            (SELECT COUNT(*) FROM deployment_sync_records WHERE project_id = $1)::bigint AS sync_record_count,
            (SELECT COUNT(*) FROM deployment_heartbeats WHERE project_id = $1)::bigint AS heartbeat_count
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let reference_counts = [
        reference_row.get::<i64, _>("config_file_count"),
        reference_row.get::<i64, _>("deployment_count"),
        reference_row.get::<i64, _>("environment_count"),
        reference_row.get::<i64, _>("release_count"),
        reference_row.get::<i64, _>("draft_count"),
        reference_row.get::<i64, _>("saved_version_count"),
        reference_row.get::<i64, _>("sync_record_count"),
        reference_row.get::<i64, _>("heartbeat_count"),
    ];

    if reference_counts.iter().any(|count| *count > 0) {
        return Err(ApiError::conflict(
            "project_delete_conflict",
            "project has dependent resources",
        ));
    }

    let project_code: String = project_row.get("code");
    let project_status: String = project_row.get("status");
    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    sqlx::query("UPDATE audit_logs SET project_id = NULL WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(actor_user_id),
            action: "project.deleted",
            resource_type: "project",
            resource_id: project_id.to_string(),
            detail: Some(serde_json::json!({
                "project_id": project_id,
                "project_code": project_code,
                "previous_status": project_status
            })),
        },
    )
    .await?;

    sqlx::query("DELETE FROM project_members WHERE project_id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?;

    let delete_result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?;
    if delete_result.rows_affected() == 0 {
        return Err(ApiError::not_found_with(
            "project_not_found",
            "project not found",
        ));
    }

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(())
}

impl CreateProjectRequest {
    fn validate(self) -> Result<ValidatedCreateProjectRequest, ApiError> {
        Ok(ValidatedCreateProjectRequest {
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            initial_admin_user_id: required_i64(
                self.initial_admin_user_id,
                "initial_admin_user_id",
            )?,
        })
    }
}

impl UpdateProjectRequest {
    fn validate(self) -> Result<ValidatedUpdateProjectRequest, ApiError> {
        Ok(ValidatedUpdateProjectRequest {
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            status: validate_status(self.status)?,
        })
    }
}

fn map_project_row(row: &sqlx::postgres::PgRow) -> ProjectSummary {
    ProjectSummary {
        id: row.get("id"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        status: row.get("status"),
        current_user_role: row.get("current_user_role"),
    }
}

fn map_project_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error
        && database_error.constraint() == Some("projects_code_key")
    {
        return ApiError::conflict("project_code_conflict", "project code already exists");
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
        "code" => "missing required body field: code",
        "name" => "missing required body field: name",
        "initial_admin_user_id" => "missing required body field: initial_admin_user_id",
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
            "invalid project status",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateProjectRequest, UpdateProjectRequest};

    #[test]
    fn create_project_request_validation_trims_required_fields() {
        let payload = CreateProjectRequest {
            code: Some(" coffee-main ".to_owned()),
            name: Some(" Coffee Main ".to_owned()),
            description: Some(" Retail ".to_owned()),
            initial_admin_user_id: Some(42),
        };

        let validated = match payload.validate() {
            Ok(validated) => validated,
            Err(error) => panic!("request should validate: {:?}", error.into_body()),
        };

        assert_eq!(validated.code, "coffee-main");
        assert_eq!(validated.name, "Coffee Main");
        assert_eq!(validated.description.as_deref(), Some("Retail"));
        assert_eq!(validated.initial_admin_user_id, 42);
    }

    #[test]
    fn update_project_request_validation_rejects_unknown_status() {
        let payload = UpdateProjectRequest {
            code: Some("coffee-main".to_owned()),
            name: Some("Coffee Main".to_owned()),
            description: None,
            status: Some("deleted".to_owned()),
        };

        let error = match payload.validate() {
            Ok(_) => panic!("unknown project status should be rejected"),
            Err(error) => error,
        };

        let body = error.into_body();
        assert_eq!(body.code, "invalid_request");
        assert_eq!(body.message, "invalid project status");
    }
}
