use crate::{
    audit::{AuditLogEntry, write_audit_log},
    authorization::require_platform_admin,
    error::ApiError,
    http::api::projects::delete_project_by_id,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use schema::admin_project::{
    CreatePlatformProjectResponse, PlatformProject, PlatformProjectInitialAdmin,
    PlatformProjectListResponse, PlatformProjectSummary,
};
use serde::Deserialize;
use sqlx::{Error as SqlxError, PgPool, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct ListAdminProjectsQuery {
    keyword: Option<String>,
    status: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreatePlatformProjectRequest {
    code: Option<String>,
    name: Option<String>,
    description: Option<String>,
    initial_admin_user_id: Option<i64>,
}

#[derive(Debug)]
struct ValidatedCreatePlatformProjectRequest {
    code: String,
    name: String,
    description: Option<String>,
    initial_admin_user_id: i64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/projects",
            get(list_admin_projects).post(create_admin_project),
        )
        .route(
            "/admin/projects/{id}",
            axum::routing::delete(delete_admin_project),
        )
}

#[utoipa::path(
    get,
    path = "/api/admin/projects",
    tag = "admin",
    params(crate::openapi::ListAdminProjectsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Platform project list", body = PlatformProjectListResponse),
        (status = 400, description = "Invalid query parameters", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_admin_projects(
    State(state): State<AppState>,
    Query(query): Query<ListAdminProjectsQuery>,
    headers: HeaderMap,
) -> Result<Json<PlatformProjectListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    require_platform_admin(pool, &headers).await?;

    let status = validate_optional_project_status(query.status)?;
    let keyword = normalize_optional(query.keyword);
    let (page, page_size) = validate_page(query.page, query.page_size)?;
    let offset = (page - 1) * page_size;

    let rows = sqlx::query(
        r#"
        SELECT
            p.id,
            p.code,
            p.name,
            p.status,
            COALESCE(member_stats.member_count, 0) AS member_count,
            COALESCE(deployment_stats.deployment_count, 0) AS deployment_count,
            to_char(p.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
            COUNT(*) OVER() AS total_count
        FROM projects p
        LEFT JOIN (
            SELECT project_id, COUNT(*)::bigint AS member_count
            FROM project_members
            GROUP BY project_id
        ) AS member_stats ON member_stats.project_id = p.id
        LEFT JOIN (
            SELECT project_id, COUNT(*)::bigint AS deployment_count
            FROM deployment_instances
            WHERE deleted_at IS NULL
            GROUP BY project_id
        ) AS deployment_stats ON deployment_stats.project_id = p.id
        WHERE ($1::varchar IS NULL OR p.code ILIKE '%' || $1 || '%' OR p.name ILIKE '%' || $1 || '%')
          AND ($2::varchar IS NULL OR p.status = $2)
        ORDER BY p.created_at DESC, p.id DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(keyword)
    .bind(status)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to list admin projects"))?;

    let total = rows
        .first()
        .map(|row| row.get::<i64, _>("total_count"))
        .unwrap_or(0);

    Ok(Json(PlatformProjectListResponse {
        items: rows.iter().map(map_platform_project_summary_row).collect(),
        total,
        page,
        page_size,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/projects",
    tag = "admin",
    request_body = crate::openapi::CreatePlatformProjectRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Platform project created", body = CreatePlatformProjectResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project code already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_admin_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreatePlatformProjectRequest>, JsonRejection>,
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

#[utoipa::path(
    delete,
    path = "/api/admin/projects/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Platform project deleted"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project has dependent resources", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_admin_project(
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
    let auth = require_platform_admin(pool, &headers).await?;

    delete_project_by_id(pool, auth.user_id, id).await?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn create_platform_project_with_initial_admin(
    pool: &PgPool,
    actor_user_id: i64,
    code: String,
    name: String,
    description: Option<String>,
    initial_admin_user_id: i64,
) -> Result<CreatePlatformProjectResponse, ApiError> {
    let target_user = sqlx::query(
        r#"
        SELECT id, username, status
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(initial_admin_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        ApiError::internal_with(
            error,
            "failed to create platform project with initial admin",
        )
    })?
    .ok_or_else(|| ApiError::not_found_with("user_not_found", "user not found"))?;

    let initial_admin_status: String = target_user.get("status");
    if initial_admin_status != "active" {
        return Err(ApiError::bad_request(
            "initial_project_admin_invalid",
            "initial project admin must be an active user",
        ));
    }

    let initial_admin_user_id: i64 = target_user.get("id");
    let initial_admin_username: String = target_user.get("username");

    let mut tx = pool.begin().await.map_err(|error| {
        ApiError::internal_with(
            error,
            "failed to create platform project with initial admin",
        )
    })?;
    let project_row = sqlx::query(
        r#"
        INSERT INTO projects (code, name, description, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id, code, name, description, status
        "#,
    )
    .bind(&code)
    .bind(&name)
    .bind(&description)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_platform_project_write_error)?;

    let project_id: i64 = project_row.get("id");

    sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, 'admin')
        "#,
    )
    .bind(project_id)
    .bind(initial_admin_user_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        ApiError::internal_with(
            error,
            "failed to create platform project with initial admin",
        )
    })?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(actor_user_id),
            action: "project.created_by_platform_admin",
            resource_type: "project",
            resource_id: project_id.to_string(),
            detail: Some(serde_json::json!({
                "project_id": project_id,
                "project_code": code,
                "initial_admin_user_id": initial_admin_user_id,
            })),
        },
    )
    .await?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(actor_user_id),
            action: "project.created_by_platform_admin",
            resource_type: "project",
            resource_id: project_id.to_string(),
            detail: Some(serde_json::json!({
                "project_id": project_id,
                "project_code": code,
                "initial_admin_user_id": initial_admin_user_id,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|error| {
        ApiError::internal_with(
            error,
            "failed to create platform project with initial admin",
        )
    })?;

    Ok(CreatePlatformProjectResponse {
        project: PlatformProject {
            id: project_id,
            code: project_row.get("code"),
            name: project_row.get("name"),
            description: project_row.get("description"),
            status: project_row.get("status"),
        },
        initial_admin: PlatformProjectInitialAdmin {
            user_id: initial_admin_user_id,
            username: initial_admin_username,
            role: "admin".to_owned(),
        },
    })
}

impl CreatePlatformProjectRequest {
    fn validate(self) -> Result<ValidatedCreatePlatformProjectRequest, ApiError> {
        Ok(ValidatedCreatePlatformProjectRequest {
            code: required_trimmed(self.code, "code")?,
            name: required_trimmed(self.name, "name")?,
            description: normalize_optional(self.description),
            initial_admin_user_id: self.initial_admin_user_id.ok_or_else(|| {
                ApiError::bad_request(
                    "initial_project_admin_required",
                    "missing required body field: initial_admin_user_id",
                )
            })?,
        })
    }
}

fn map_platform_project_summary_row(row: &sqlx::postgres::PgRow) -> PlatformProjectSummary {
    PlatformProjectSummary {
        id: row.get("id"),
        code: row.get("code"),
        name: row.get("name"),
        status: row.get("status"),
        member_count: row.get("member_count"),
        deployment_count: row.get("deployment_count"),
        created_at: row.get("created_at"),
    }
}

fn map_platform_project_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error
        && database_error.constraint() == Some("projects_code_key")
    {
        return ApiError::conflict("project_code_conflict", "project code already exists");
    }

    ApiError::internal_with(error, "failed to write platform project")
}

fn validate_optional_project_status(value: Option<String>) -> Result<Option<String>, ApiError> {
    value.map(validate_project_status).transpose()
}

fn validate_project_status(value: String) -> Result<String, ApiError> {
    match value.as_str() {
        "active" | "archived" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid project status",
        )),
    }
}

fn required_trimmed(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
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

fn validate_page(page: Option<i64>, page_size: Option<i64>) -> Result<(i64, i64), ApiError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    if page < 1 {
        return Err(ApiError::bad_request(
            "invalid_request",
            "page must be greater than or equal to 1",
        ));
    }
    if !(1..=100).contains(&page_size) {
        return Err(ApiError::bad_request(
            "invalid_request",
            "page_size must be between 1 and 100",
        ));
    }

    Ok((page, page_size))
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "code" => "missing required body field: code",
        "name" => "missing required body field: name",
        _ => "missing required body field",
    }
}
