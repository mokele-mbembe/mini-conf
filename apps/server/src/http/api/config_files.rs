use crate::{
    audit::{AuditLogEntry, write_audit_log},
    authorization::{ProjectRole, authenticate_user, require_project_role},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use schema::config_file::{ConfigFileListResponse, ConfigFileSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
pub(crate) struct ListConfigFilesQuery {
    project_id: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateConfigFileRequest {
    project_id: Option<i64>,
    code: Option<String>,
    name: Option<String>,
    is_required: Option<bool>,
    format: Option<String>,
    sensitivity: Option<String>,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
}

#[derive(Debug)]
struct ValidatedCreateConfigFileRequest {
    project_id: i64,
    code: String,
    name: String,
    is_required: bool,
    format: String,
    sensitivity: String,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateConfigFileRequest {
    project_id: Option<i64>,
    code: Option<String>,
    name: Option<String>,
    is_required: Option<bool>,
    format: Option<String>,
    sensitivity: Option<String>,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
    status: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateConfigFileRequest {
    project_id: i64,
    code: String,
    name: String,
    is_required: bool,
    format: String,
    sensitivity: String,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/config-files",
            get(list_config_files).post(create_config_file),
        )
        .route(
            "/config-files/{id}",
            get(get_config_file).put(update_config_file),
        )
}

#[utoipa::path(
    get,
    path = "/api/config-files",
    tag = "admin",
    params(crate::openapi::ListConfigFilesParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List config files", body = ConfigFileListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_config_files(
    State(state): State<AppState>,
    Query(query): Query<ListConfigFilesQuery>,
    headers: HeaderMap,
) -> Result<Json<ConfigFileListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            cf.id,
            cf.project_id,
            cf.code,
            cf.name,
            cf.is_required,
            cf.format,
            cf.sensitivity,
            cf.secret_paths,
            cf.description,
            cf.status
        FROM config_files cf
        JOIN project_members pm
          ON pm.project_id = cf.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR cf.project_id = $2)
          AND (
                $3::varchar IS NULL
                AND cf.status IN ('active', 'archived')
              OR
                $3::varchar IS NOT NULL
                AND cf.status = $3
          )
        ORDER BY cf.project_id ASC, cf.code ASC, cf.id ASC
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(normalize_optional(query.status))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(ConfigFileListResponse {
        items: rows.into_iter().map(map_config_file_row).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/config-files",
    tag = "admin",
    request_body = crate::openapi::CreateConfigFileRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Config file created", body = ConfigFileSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Config file code already exists within the project", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_config_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateConfigFileRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ConfigFileSummary>), ApiError> {
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
        payload.project_id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            is_required,
            format,
            sensitivity,
            secret_paths,
            description,
            status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'active')
        RETURNING
            id,
            project_id,
            code,
            name,
            is_required,
            format,
            sensitivity,
            secret_paths,
            description,
            status
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.is_required)
    .bind(payload.format)
    .bind(payload.sensitivity)
    .bind(payload.secret_paths.map(SqlxJson))
    .bind(payload.description)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_config_file_write_error)?;

    let summary = map_config_file_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "config_file.created",
            resource_type: "config_file",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "config_file_id": summary.id,
                "changed_fields": ["code", "name", "is_required", "format", "sensitivity", "description"]
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(summary)))
}

#[utoipa::path(
    get,
    path = "/api/config-files/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Config file ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Config file detail", body = ConfigFileSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Config file not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_config_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<ConfigFileSummary>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let row = sqlx::query(
        r#"
        SELECT
            cf.id,
            cf.project_id,
            cf.code,
            cf.name,
            cf.is_required,
            cf.format,
            cf.sensitivity,
            cf.secret_paths,
            cf.description,
            cf.status
        FROM config_files cf
        JOIN project_members pm
          ON pm.project_id = cf.project_id
         AND pm.user_id = $2
        WHERE cf.id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("config_file_not_found", "config file not found"))?;

    Ok(Json(map_config_file_row(row)))
}

#[utoipa::path(
    put,
    path = "/api/config-files/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Config file ID")
    ),
    request_body = crate::openapi::UpdateConfigFileRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Config file updated", body = ConfigFileSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or config file not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Config file code already exists within the project", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_config_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateConfigFileRequest>, JsonRejection>,
) -> Result<Json<ConfigFileSummary>, ApiError> {
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
    let existing_project_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT project_id
        FROM config_files
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("config_file_not_found", "config file not found"))?;

    require_project_role(
        pool,
        auth.user_id,
        existing_project_id,
        ProjectRole::Admin,
        "config_file_not_found",
        "config file not found",
    )
    .await?;
    if payload.project_id != existing_project_id {
        require_project_role(
            pool,
            auth.user_id,
            payload.project_id,
            ProjectRole::Admin,
            "project_not_found",
            "project not found",
        )
        .await?;
    }

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        UPDATE config_files
        SET
            project_id = $2,
            code = $3,
            name = $4,
            is_required = $5,
            format = $6,
            sensitivity = $7,
            secret_paths = $8,
            description = $9,
            status = $10,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            project_id,
            code,
            name,
            is_required,
            format,
            sensitivity,
            secret_paths,
            description,
            status
        "#,
    )
    .bind(id)
    .bind(payload.project_id)
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.is_required)
    .bind(payload.format)
    .bind(payload.sensitivity)
    .bind(payload.secret_paths.map(SqlxJson))
    .bind(payload.description)
    .bind(payload.status)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_config_file_write_error)?
    .ok_or_else(|| ApiError::not_found_with("config_file_not_found", "config file not found"))?;

    let summary = map_config_file_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "config_file.updated",
            resource_type: "config_file",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "config_file_id": summary.id,
                "changed_fields": ["code", "name", "is_required", "format", "sensitivity", "description", "status"]
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(summary))
}

impl CreateConfigFileRequest {
    fn validate(self) -> Result<ValidatedCreateConfigFileRequest, ApiError> {
        Ok(ValidatedCreateConfigFileRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            is_required: self.is_required.unwrap_or(false),
            format: validate_format(self.format)?,
            sensitivity: validate_sensitivity(self.sensitivity)?,
            secret_paths: normalize_secret_paths(self.secret_paths),
            description: normalize_optional(self.description),
        })
    }
}

impl UpdateConfigFileRequest {
    fn validate(self) -> Result<ValidatedUpdateConfigFileRequest, ApiError> {
        Ok(ValidatedUpdateConfigFileRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            is_required: self.is_required.unwrap_or(false),
            format: validate_format(self.format)?,
            sensitivity: validate_sensitivity(self.sensitivity)?,
            secret_paths: normalize_secret_paths(self.secret_paths),
            description: normalize_optional(self.description),
            status: validate_status(self.status)?,
        })
    }
}

fn map_config_file_row(row: sqlx::postgres::PgRow) -> ConfigFileSummary {
    let secret_paths = row
        .get::<Option<SqlxJson<Vec<String>>>, _>("secret_paths")
        .map(|value| value.0);

    ConfigFileSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        code: row.get("code"),
        name: row.get("name"),
        is_required: row.get("is_required"),
        format: row.get("format"),
        sensitivity: row.get("sensitivity"),
        secret_paths,
        description: row.get("description"),
        status: row.get("status"),
    }
}

fn map_config_file_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error {
        if database_error.constraint() == Some("config_files_project_id_code_key") {
            return ApiError::conflict(
                "config_file_code_conflict",
                "config file code already exists in project",
            );
        }

        if database_error.constraint() == Some("config_files_project_id_fkey") {
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

fn normalize_secret_paths(value: Option<Vec<String>>) -> Option<Vec<String>> {
    value.and_then(|items| {
        let items = items
            .into_iter()
            .filter_map(|item| {
                let trimmed = item.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            })
            .collect::<Vec<_>>();

        if items.is_empty() { None } else { Some(items) }
    })
}

fn validate_sensitivity(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Ok("normal".to_owned());
    };

    match value.as_str() {
        "normal" | "secret" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid config file sensitivity",
        )),
    }
}

fn validate_format(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message("format"),
        ));
    };

    match value.as_str() {
        "yaml" | "json" | "toml" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid config file format",
        )),
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
            "invalid config file status",
        )),
    }
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "project_id" => "missing required body field: project_id",
        "code" => "missing required body field: code",
        "name" => "missing required body field: name",
        "format" => "missing required body field: format",
        "status" => "missing required body field: status",
        _ => "missing required body field",
    }
}
