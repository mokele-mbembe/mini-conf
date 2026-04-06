use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    routing::get,
};
use schema::config_file::{ConfigFileListResponse, ConfigFileSummary};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
pub(crate) struct ListConfigFilesQuery {
    project_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateConfigFileRequest {
    project_id: Option<i64>,
    code: Option<String>,
    name: Option<String>,
    format: Option<String>,
    schema_name: Option<String>,
    schema_version: Option<String>,
    sensitivity: Option<String>,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
}

#[derive(Debug)]
struct ValidatedCreateConfigFileRequest {
    project_id: i64,
    code: String,
    name: String,
    format: String,
    schema_name: Option<String>,
    schema_version: Option<String>,
    sensitivity: String,
    secret_paths: Option<Vec<String>>,
    description: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/config-files",
        get(list_config_files).post(create_config_file),
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
            code,
            name,
            format,
            schema_name,
            schema_version,
            sensitivity,
            secret_paths,
            description,
            status
        FROM config_files
        WHERE status = 'active'
          AND ($1::bigint IS NULL OR project_id = $1)
        ORDER BY project_id ASC, code ASC, id ASC
        "#,
    )
    .bind(query.project_id)
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

    authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    let row = sqlx::query(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            format,
            schema_name,
            schema_version,
            sensitivity,
            secret_paths,
            description,
            status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
        RETURNING
            id,
            project_id,
            code,
            name,
            format,
            schema_name,
            schema_version,
            sensitivity,
            secret_paths,
            description,
            status
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.code)
    .bind(payload.name)
    .bind(payload.format)
    .bind(payload.schema_name)
    .bind(payload.schema_version)
    .bind(payload.sensitivity)
    .bind(payload.secret_paths.map(SqlxJson))
    .bind(payload.description)
    .fetch_one(pool)
    .await
    .map_err(map_config_file_write_error)?;

    Ok((StatusCode::CREATED, Json(map_config_file_row(row))))
}

impl CreateConfigFileRequest {
    fn validate(self) -> Result<ValidatedCreateConfigFileRequest, ApiError> {
        Ok(ValidatedCreateConfigFileRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            code: required(self.code, "code")?,
            name: required(self.name, "name")?,
            format: required(self.format, "format")?,
            schema_name: normalize_optional(self.schema_name),
            schema_version: normalize_optional(self.schema_version),
            sensitivity: validate_sensitivity(self.sensitivity)?,
            secret_paths: normalize_secret_paths(self.secret_paths),
            description: normalize_optional(self.description),
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
        format: row.get("format"),
        schema_name: row.get("schema_name"),
        schema_version: row.get("schema_version"),
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

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "project_id" => "missing required body field: project_id",
        "code" => "missing required body field: code",
        "name" => "missing required body field: name",
        "format" => "missing required body field: format",
        _ => "missing required body field",
    }
}
