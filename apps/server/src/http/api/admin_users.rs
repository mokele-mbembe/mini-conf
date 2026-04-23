use crate::{
    audit::{AuditLogEntry, write_audit_log},
    auth::{hash_password, validate_password_strength},
    authorization::require_platform_admin,
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use schema::admin_user::{
    AdminUserDetail, AdminUserListResponse, AdminUserProjectSummary, AdminUserSummary,
};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct ListAdminUsersQuery {
    keyword: Option<String>,
    status: Option<String>,
    is_platform_admin: Option<bool>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateAdminUserRequest {
    username: Option<String>,
    password: Option<String>,
    is_platform_admin: Option<bool>,
    must_change_password: Option<bool>,
    status: Option<String>,
}

#[derive(Debug)]
struct ValidatedCreateAdminUserRequest {
    username: String,
    password: String,
    is_platform_admin: bool,
    must_change_password: bool,
    status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateAdminUserRequest {
    status: Option<String>,
    is_platform_admin: Option<bool>,
    must_change_password: Option<bool>,
}

#[derive(Debug)]
struct ValidatedUpdateAdminUserRequest {
    status: Option<String>,
    is_platform_admin: Option<bool>,
    must_change_password: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResetAdminUserPasswordRequest {
    new_password: Option<String>,
    must_change_password: Option<bool>,
}

#[derive(Debug)]
struct ValidatedResetAdminUserPasswordRequest {
    new_password: String,
    must_change_password: bool,
}

#[derive(Debug)]
struct ExistingAdminUser {
    id: i64,
    username: String,
    status: String,
    is_platform_admin: bool,
    must_change_password: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/users",
            get(list_admin_users).post(create_admin_user),
        )
        .route(
            "/admin/users/{id}",
            get(get_admin_user).patch(update_admin_user),
        )
        .route(
            "/admin/users/{id}/reset-password",
            post(reset_admin_user_password),
        )
}

#[utoipa::path(
    get,
    path = "/api/admin/users",
    tag = "admin",
    params(crate::openapi::ListAdminUsersParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Platform admin user list", body = AdminUserListResponse),
        (status = 400, description = "Invalid query parameters", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_admin_users(
    State(state): State<AppState>,
    Query(query): Query<ListAdminUsersQuery>,
    headers: HeaderMap,
) -> Result<Json<AdminUserListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    require_platform_admin(pool, &headers).await?;

    let status = validate_optional_user_status(query.status)?;
    let keyword = normalize_optional(query.keyword);
    let (page, page_size) = validate_page(query.page, query.page_size)?;
    let offset = (page - 1) * page_size;

    let rows = sqlx::query(
        r#"
        SELECT
            u.id,
            u.username,
            u.status,
            u.is_platform_admin,
            u.must_change_password,
            to_char(u.last_login_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS last_login_at,
            to_char(u.password_updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS password_updated_at,
            COALESCE(member_stats.project_count, 0) AS project_count,
            to_char(u.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at,
            COUNT(*) OVER() AS total_count
        FROM users u
        LEFT JOIN (
            SELECT user_id, COUNT(*)::bigint AS project_count
            FROM project_members
            GROUP BY user_id
        ) AS member_stats ON member_stats.user_id = u.id
        WHERE ($1::varchar IS NULL OR u.username ILIKE '%' || $1 || '%')
          AND ($2::varchar IS NULL OR u.status = $2)
          AND ($3::boolean IS NULL OR u.is_platform_admin = $3)
        ORDER BY u.created_at DESC, u.id DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(keyword)
    .bind(status)
    .bind(query.is_platform_admin)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let total = rows
        .first()
        .map(|row| row.get::<i64, _>("total_count"))
        .unwrap_or(0);

    Ok(Json(AdminUserListResponse {
        items: rows.iter().map(map_admin_user_summary_row).collect(),
        total,
        page,
        page_size,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/users",
    tag = "admin",
    request_body = crate::openapi::CreateAdminUserRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Admin user created", body = AdminUserSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 409, description = "Username already exists", body = crate::error::ErrorResponse),
        (status = 422, description = "Password does not meet strength requirements", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_admin_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateAdminUserRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<AdminUserSummary>), ApiError> {
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
    validate_password_strength(&payload.password)?;
    let password_hash = hash_password(&payload.password)?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        INSERT INTO users (
            username,
            password_hash,
            status,
            is_platform_admin,
            must_change_password,
            password_updated_at
        )
        VALUES ($1, $2, $3, $4, $5, NOW())
        RETURNING
            id,
            username,
            status,
            is_platform_admin,
            must_change_password,
            NULL::varchar AS last_login_at,
            to_char(password_updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS password_updated_at,
            0::bigint AS project_count,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        "#,
    )
    .bind(&payload.username)
    .bind(password_hash)
    .bind(&payload.status)
    .bind(payload.is_platform_admin)
    .bind(payload.must_change_password)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_admin_user_write_error)?;

    let user_id: i64 = row.get("id");
    let username: String = row.get("username");

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(auth.user_id),
            action: "user.created",
            resource_type: "user",
            resource_id: user_id.to_string(),
            detail: Some(serde_json::json!({
                "target_user_id": user_id,
                "target_username": username,
                "changed_fields": ["status", "is_platform_admin", "must_change_password", "password"],
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(map_admin_user_summary_row(&row))))
}

#[utoipa::path(
    get,
    path = "/api/admin/users/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Admin user detail", body = AdminUserDetail),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_admin_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<AdminUserDetail>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    require_platform_admin(pool, &headers).await?;

    let row = sqlx::query(
        r#"
        SELECT
            u.id,
            u.username,
            u.status,
            u.is_platform_admin,
            u.must_change_password,
            to_char(u.last_login_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS last_login_at,
            to_char(u.password_updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS password_updated_at,
            to_char(u.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM users u
        WHERE u.id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("user_not_found", "user not found"))?;

    let project_rows = sqlx::query(
        r#"
        SELECT p.id AS project_id, p.code AS project_code, p.name AS project_name, pm.role
        FROM project_members pm
        JOIN projects p ON p.id = pm.project_id
        WHERE pm.user_id = $1
        ORDER BY p.code ASC, p.id ASC
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(AdminUserDetail {
        id: row.get("id"),
        username: row.get("username"),
        status: row.get("status"),
        is_platform_admin: row.get("is_platform_admin"),
        must_change_password: row.get("must_change_password"),
        last_login_at: row.get("last_login_at"),
        password_updated_at: row.get("password_updated_at"),
        projects: project_rows
            .iter()
            .map(|project_row| AdminUserProjectSummary {
                project_id: project_row.get("project_id"),
                project_code: project_row.get("project_code"),
                project_name: project_row.get("project_name"),
                role: project_row.get("role"),
            })
            .collect(),
        created_at: row.get("created_at"),
    }))
}

#[utoipa::path(
    patch,
    path = "/api/admin/users/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = crate::openapi::UpdateAdminUserRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Admin user updated", body = AdminUserSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 409, description = "The platform must keep at least one active platform admin", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_admin_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateAdminUserRequest>, JsonRejection>,
) -> Result<Json<AdminUserSummary>, ApiError> {
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
    let existing = load_existing_admin_user(pool, id).await?;

    let target_status = payload
        .status
        .clone()
        .unwrap_or_else(|| existing.status.clone());
    let target_is_platform_admin = payload
        .is_platform_admin
        .unwrap_or(existing.is_platform_admin);
    let target_must_change_password = payload
        .must_change_password
        .unwrap_or(existing.must_change_password);

    ensure_last_platform_admin_rule(pool, &existing, target_is_platform_admin, &target_status)
        .await?;

    let changed_fields = collect_changed_user_fields(
        &existing,
        &target_status,
        target_is_platform_admin,
        target_must_change_password,
    );

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        UPDATE users
        SET
            status = $2,
            is_platform_admin = $3,
            must_change_password = $4,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            username,
            status,
            is_platform_admin,
            must_change_password,
            to_char(last_login_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS last_login_at,
            to_char(password_updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS password_updated_at,
            (
                SELECT COUNT(*)::bigint
                FROM project_members pm
                WHERE pm.user_id = users.id
            ) AS project_count,
            to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        "#,
    )
    .bind(id)
    .bind(&target_status)
    .bind(target_is_platform_admin)
    .bind(target_must_change_password)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    if existing.status == "active" && target_status == "disabled" {
        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET status = 'revoked', updated_at = NOW()
            WHERE user_id = $1
              AND status = 'active'
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?;
    }

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(auth.user_id),
            action: audit_action_for_user_status_change(&existing.status, &target_status),
            resource_type: "user",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "changed_fields": changed_fields,
                "target_user_id": existing.id,
                "target_username": existing.username,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(map_admin_user_summary_row(&row)))
}

#[utoipa::path(
    post,
    path = "/api/admin/users/{id}/reset-password",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    request_body = crate::openapi::ResetAdminUserPasswordRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Admin user password reset"),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 404, description = "User not found", body = crate::error::ErrorResponse),
        (status = 422, description = "Password does not meet strength requirements", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn reset_admin_user_password(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<ResetAdminUserPasswordRequest>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
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
    let existing = load_existing_admin_user(pool, id).await?;

    validate_password_strength(&payload.new_password)?;
    let password_hash = hash_password(&payload.new_password)?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    sqlx::query(
        r#"
        UPDATE users
        SET
            password_hash = $2,
            must_change_password = $3,
            password_updated_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(password_hash)
    .bind(payload.must_change_password)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET status = 'revoked', updated_at = NOW()
        WHERE user_id = $1
          AND status = 'active'
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: None,
            user_id: Some(auth.user_id),
            action: "user.password_reset",
            resource_type: "user",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "changed_fields": ["password", "must_change_password"],
                "target_user_id": existing.id,
                "target_username": existing.username,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(StatusCode::NO_CONTENT)
}

impl CreateAdminUserRequest {
    fn validate(self) -> Result<ValidatedCreateAdminUserRequest, ApiError> {
        Ok(ValidatedCreateAdminUserRequest {
            username: required_trimmed(self.username, "username")?,
            password: required_password(self.password, "password")?,
            is_platform_admin: self.is_platform_admin.unwrap_or(false),
            must_change_password: self.must_change_password.unwrap_or(false),
            status: validate_user_status(required_trimmed(self.status, "status")?)?,
        })
    }
}

impl UpdateAdminUserRequest {
    fn validate(self) -> Result<ValidatedUpdateAdminUserRequest, ApiError> {
        if self.status.is_none()
            && self.is_platform_admin.is_none()
            && self.must_change_password.is_none()
        {
            return Err(ApiError::bad_request(
                "invalid_request",
                "at least one field must be provided",
            ));
        }

        Ok(ValidatedUpdateAdminUserRequest {
            status: validate_optional_user_status(self.status)?,
            is_platform_admin: self.is_platform_admin,
            must_change_password: self.must_change_password,
        })
    }
}

impl ResetAdminUserPasswordRequest {
    fn validate(self) -> Result<ValidatedResetAdminUserPasswordRequest, ApiError> {
        Ok(ValidatedResetAdminUserPasswordRequest {
            new_password: required_password(self.new_password, "new_password")?,
            must_change_password: self.must_change_password.unwrap_or(true),
        })
    }
}

async fn load_existing_admin_user(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<ExistingAdminUser, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id, username, status, is_platform_admin, must_change_password
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("user_not_found", "user not found"))?;

    Ok(ExistingAdminUser {
        id: row.get("id"),
        username: row.get("username"),
        status: row.get("status"),
        is_platform_admin: row.get("is_platform_admin"),
        must_change_password: row.get("must_change_password"),
    })
}

async fn ensure_last_platform_admin_rule(
    pool: &sqlx::PgPool,
    existing: &ExistingAdminUser,
    target_is_platform_admin: bool,
    target_status: &str,
) -> Result<(), ApiError> {
    let was_active_platform_admin = existing.is_platform_admin && existing.status == "active";
    let remains_active_platform_admin = target_is_platform_admin && target_status == "active";

    if !was_active_platform_admin || remains_active_platform_admin {
        return Ok(());
    }

    let active_platform_admin_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM users
        WHERE is_platform_admin = TRUE
          AND status = 'active'
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    if active_platform_admin_count <= 1 {
        return Err(ApiError::conflict(
            "last_platform_admin_required",
            "platform must keep at least one active platform admin",
        ));
    }

    Ok(())
}

fn map_admin_user_summary_row(row: &sqlx::postgres::PgRow) -> AdminUserSummary {
    AdminUserSummary {
        id: row.get("id"),
        username: row.get("username"),
        status: row.get("status"),
        is_platform_admin: row.get("is_platform_admin"),
        must_change_password: row.get("must_change_password"),
        last_login_at: row.get("last_login_at"),
        password_updated_at: row.get("password_updated_at"),
        project_count: row.get("project_count"),
        created_at: row.get("created_at"),
    }
}

fn map_admin_user_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error
        && database_error.constraint() == Some("users_username_key")
    {
        return ApiError::conflict("user_username_conflict", "username already exists");
    }

    ApiError::internal()
}

fn collect_changed_user_fields(
    existing: &ExistingAdminUser,
    target_status: &str,
    target_is_platform_admin: bool,
    target_must_change_password: bool,
) -> Vec<&'static str> {
    let mut changed_fields = Vec::new();

    if existing.status != target_status {
        changed_fields.push("status");
    }
    if existing.is_platform_admin != target_is_platform_admin {
        changed_fields.push("is_platform_admin");
    }
    if existing.must_change_password != target_must_change_password {
        changed_fields.push("must_change_password");
    }

    changed_fields
}

fn audit_action_for_user_status_change(old_status: &str, new_status: &str) -> &'static str {
    match (old_status, new_status) {
        ("active", "disabled") => "user.disabled",
        ("disabled", "active") => "user.enabled",
        _ => "user.updated",
    }
}

fn validate_optional_user_status(value: Option<String>) -> Result<Option<String>, ApiError> {
    value.map(validate_user_status).transpose()
}

fn validate_user_status(value: String) -> Result<String, ApiError> {
    match value.as_str() {
        "active" | "disabled" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid user status",
        )),
    }
}

fn required_trimmed(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
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

fn required_password(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
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
        "username" => "missing required body field: username",
        "password" => "missing required body field: password",
        "status" => "missing required body field: status",
        "new_password" => "missing required body field: new_password",
        _ => "missing required body field",
    }
}
