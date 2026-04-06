use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    http::{HeaderMap, header},
    routing::get,
};
use schema::project::{ProjectListResponse, ProjectSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateProjectRequest {
    code: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct ValidatedCreateProjectRequest {
    code: String,
    name: String,
    description: Option<String>,
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
        .route("/projects/{id}", get(get_project).put(update_project))
}

#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "admin",
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List active projects visible to the current admin session", body = ProjectListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_projects(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProjectListResponse>, ApiError> {
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
        SELECT id, code, name, description, status
        FROM projects
        WHERE status = 'active'
        ORDER BY code ASC, id ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let items = rows
        .into_iter()
        .map(|row| ProjectSummary {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
        })
        .collect();

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

    authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    let row = sqlx::query(
        r#"
        SELECT id, code, name, description, status
        FROM projects
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("project_not_found", "project not found"))?;

    Ok(Json(ProjectSummary {
        id: row.get("id"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        status: row.get("status"),
    }))
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

    authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

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
        RETURNING id, code, name, description, status
        "#,
    )
    .bind(id)
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.status)
    .fetch_optional(pool)
    .await
    .map_err(map_project_write_error)?
    .ok_or_else(|| ApiError::not_found_with("project_not_found", "project not found"))?;

    Ok(Json(ProjectSummary {
        id: row.get("id"),
        code: row.get("code"),
        name: row.get("name"),
        description: row.get("description"),
        status: row.get("status"),
    }))
}

#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "admin",
    request_body = crate::openapi::CreateProjectRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Project created", body = ProjectSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 409, description = "Project code already exists", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_project(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateProjectRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProjectSummary>), ApiError> {
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
        INSERT INTO projects (code, name, description, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id, code, name, description, status
        "#,
    )
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.description)
    .fetch_one(pool)
    .await
    .map_err(map_project_write_error)?;

    Ok((
        StatusCode::CREATED,
        Json(ProjectSummary {
            id: row.get("id"),
            code: row.get("code"),
            name: row.get("name"),
            description: row.get("description"),
            status: row.get("status"),
        }),
    ))
}

impl CreateProjectRequest {
    fn validate(self) -> Result<ValidatedCreateProjectRequest, ApiError> {
        Ok(ValidatedCreateProjectRequest {
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
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
