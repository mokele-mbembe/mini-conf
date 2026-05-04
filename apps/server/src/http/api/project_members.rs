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
use schema::project_member::{ProjectMemberListResponse, ProjectMemberSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateProjectMemberRequest {
    username: Option<String>,
    role: Option<String>,
}

#[derive(Debug)]
struct ValidatedCreateProjectMemberRequest {
    username: String,
    role: ProjectRole,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateProjectMemberRequest {
    role: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateProjectMemberRequest {
    role: ProjectRole,
}

#[derive(Debug)]
struct ExistingProjectMember {
    username: String,
    role: ProjectRole,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{id}/members",
            get(list_project_members).post(create_project_member),
        )
        .route(
            "/projects/{id}/members/{member_id}",
            axum::routing::put(update_project_member).delete(delete_project_member),
        )
}

#[utoipa::path(
    get,
    path = "/api/projects/{id}/members",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project member list", body = ProjectMemberListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current member cannot manage project members", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_project_members(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ProjectMemberListResponse>, ApiError> {
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

    let rows = sqlx::query(
        r#"
        SELECT
            pm.id,
            pm.project_id,
            pm.user_id,
            u.username,
            pm.role,
            to_char(pm.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM project_members pm
        JOIN users u ON u.id = pm.user_id
        WHERE pm.project_id = $1
        ORDER BY
            CASE pm.role
                WHEN 'admin' THEN 1
                WHEN 'editor' THEN 2
                ELSE 3
            END,
            u.username ASC,
            pm.id ASC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to list project members"))?;

    Ok(Json(ProjectMemberListResponse {
        items: rows.iter().map(map_project_member_row).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/projects/{id}/members",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID")
    ),
    request_body = crate::openapi::CreateProjectMemberRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Project member created", body = ProjectMemberSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current member cannot manage project members", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or target user not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Project member already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_project_member(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<CreateProjectMemberRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProjectMemberSummary>), ApiError> {
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

    let target = sqlx::query(
        r#"
        SELECT id, username
        FROM users
        WHERE username = $1
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(&payload.username)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to create project member"))?
    .ok_or_else(|| ApiError::not_found_with("user_not_found", "user not found"))?;

    let target_user_id: i64 = target.get("id");
    let target_username: String = target.get("username");

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to create project member"))?;
    let row = sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING
            id,
            project_id,
            user_id,
            $4::varchar AS username,
            role,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        "#,
    )
    .bind(project_id)
    .bind(target_user_id)
    .bind(payload.role.as_str())
    .bind(&target_username)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_project_member_write_error)?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_member.created",
            resource_type: "project_member",
            resource_id: row.get::<i64, _>("id").to_string(),
            detail: Some(serde_json::json!({
                "username": target_username,
                "role": payload.role.as_str(),
            })),
        },
    )
    .await?;

    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to create project member"))?;

    Ok((StatusCode::CREATED, Json(map_project_member_row(&row))))
}

#[utoipa::path(
    put,
    path = "/api/projects/{id}/members/{member_id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID"),
        ("member_id" = i64, Path, description = "Project member ID")
    ),
    request_body = crate::openapi::UpdateProjectMemberRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Project member updated", body = ProjectMemberSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current member cannot manage project members", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or project member not found", body = crate::error::ErrorResponse),
        (status = 409, description = "The last project admin cannot be removed or downgraded", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_project_member(
    State(state): State<AppState>,
    Path((project_id, member_id)): Path<(i64, i64)>,
    headers: HeaderMap,
    payload: Result<Json<UpdateProjectMemberRequest>, JsonRejection>,
) -> Result<Json<ProjectMemberSummary>, ApiError> {
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

    let existing = load_existing_project_member(pool, project_id, member_id).await?;
    ensure_last_admin_rule(pool, project_id, &existing, payload.role).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to update project member"))?;
    let row = sqlx::query(
        r#"
        UPDATE project_members
        SET role = $3
        WHERE id = $1
          AND project_id = $2
        RETURNING
            id,
            project_id,
            user_id,
            $4::varchar AS username,
            role,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        "#,
    )
    .bind(member_id)
    .bind(project_id)
    .bind(payload.role.as_str())
    .bind(&existing.username)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to update project member"))?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_member.updated",
            resource_type: "project_member",
            resource_id: member_id.to_string(),
            detail: Some(serde_json::json!({
                "username": existing.username,
                "role": payload.role.as_str(),
            })),
        },
    )
    .await?;

    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to update project member"))?;

    Ok(Json(map_project_member_row(&row)))
}

#[utoipa::path(
    delete,
    path = "/api/projects/{id}/members/{member_id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Project ID"),
        ("member_id" = i64, Path, description = "Project member ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Project member deleted"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current member cannot manage project members", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or project member not found", body = crate::error::ErrorResponse),
        (status = 409, description = "The last project admin cannot be removed or downgraded", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_project_member(
    State(state): State<AppState>,
    Path((project_id, member_id)): Path<(i64, i64)>,
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

    let existing = load_existing_project_member(pool, project_id, member_id).await?;
    ensure_last_admin_rule(pool, project_id, &existing, ProjectRole::Viewer).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to delete project member"))?;
    sqlx::query(
        r#"
        DELETE FROM project_members
        WHERE id = $1
          AND project_id = $2
        "#,
    )
    .bind(member_id)
    .bind(project_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to delete project member"))?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "project_member.deleted",
            resource_type: "project_member",
            resource_id: member_id.to_string(),
            detail: Some(serde_json::json!({
                "username": existing.username,
                "role": existing.role.as_str(),
            })),
        },
    )
    .await?;

    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to delete project member"))?;

    Ok(StatusCode::NO_CONTENT)
}

impl CreateProjectMemberRequest {
    fn validate(self) -> Result<ValidatedCreateProjectMemberRequest, ApiError> {
        Ok(ValidatedCreateProjectMemberRequest {
            username: required(self.username, "username")?,
            role: ProjectRole::parse(required(self.role, "role")?.as_str())?,
        })
    }
}

impl UpdateProjectMemberRequest {
    fn validate(self) -> Result<ValidatedUpdateProjectMemberRequest, ApiError> {
        Ok(ValidatedUpdateProjectMemberRequest {
            role: ProjectRole::parse(required(self.role, "role")?.as_str())?,
        })
    }
}

async fn load_existing_project_member(
    pool: &sqlx::PgPool,
    project_id: i64,
    member_id: i64,
) -> Result<ExistingProjectMember, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT pm.id, pm.user_id, u.username, pm.role
        FROM project_members pm
        JOIN users u ON u.id = pm.user_id
        WHERE pm.project_id = $1
          AND pm.id = $2
        LIMIT 1
        "#,
    )
    .bind(project_id)
    .bind(member_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to load existing project member"))?
    .ok_or_else(|| {
        ApiError::not_found_with("project_member_not_found", "project member not found")
    })?;

    Ok(ExistingProjectMember {
        username: row.get("username"),
        role: ProjectRole::parse(row.get::<String, _>("role").as_str())?,
    })
}

async fn ensure_last_admin_rule(
    pool: &sqlx::PgPool,
    project_id: i64,
    existing: &ExistingProjectMember,
    target_role: ProjectRole,
) -> Result<(), ApiError> {
    if existing.role != ProjectRole::Admin || target_role == ProjectRole::Admin {
        return Ok(());
    }

    let admin_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM project_members
        WHERE project_id = $1
          AND role = 'admin'
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to ensure last admin rule"))?;

    if admin_count <= 1 {
        return Err(ApiError::conflict(
            "last_project_admin_required",
            "project must keep at least one admin",
        ));
    }

    Ok(())
}

fn map_project_member_row(row: &sqlx::postgres::PgRow) -> ProjectMemberSummary {
    ProjectMemberSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        user_id: row.get("user_id"),
        username: row.get("username"),
        role: row.get("role"),
        created_at: row.get("created_at"),
    }
}

fn map_project_member_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error
        && database_error.constraint() == Some("project_members_project_id_user_id_key")
    {
        return ApiError::conflict("project_member_conflict", "project member already exists");
    }

    ApiError::internal_with(error, "failed to write project member")
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = value else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    }

    Ok(trimmed.to_owned())
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "username" => "missing required body field: username",
        "role" => "missing required body field: role",
        _ => "missing required body field",
    }
}
