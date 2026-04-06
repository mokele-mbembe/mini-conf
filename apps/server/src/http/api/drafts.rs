use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, header},
    routing::get,
};
use schema::draft::DraftResponse;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateDraftRequest {
    content: Option<String>,
    format: Option<String>,
    base_version: Option<i64>,
}

#[derive(Debug)]
struct ValidatedUpdateDraftRequest {
    content: String,
    format: String,
    base_version: Option<i64>,
}

#[derive(Debug)]
struct DraftContext {
    project_id: i64,
    format: String,
    schema_version: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/drafts/{deployment_id}/{config_file_id}",
        get(get_draft).put(put_draft),
    )
}

#[utoipa::path(
    get,
    path = "/api/drafts/{deployment_id}/{config_file_id}",
    tag = "admin",
    params(
        ("deployment_id" = i64, Path, description = "Deployment instance ID"),
        ("config_file_id" = i64, Path, description = "Config file ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Draft detail", body = DraftResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance, config file, or draft not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_draft(
    State(state): State<AppState>,
    Path((deployment_id, config_file_id)): Path<(i64, i64)>,
    headers: HeaderMap,
) -> Result<Json<DraftResponse>, ApiError> {
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

    load_draft_context(pool, deployment_id, config_file_id).await?;

    let row = sqlx::query(
        r#"
        SELECT
            deployment_instance_id,
            config_file_id,
            format,
            content,
            version,
            schema_version,
            to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
        FROM drafts
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("draft_not_found", "draft not found"))?;

    Ok(Json(map_draft_row(row)))
}

#[utoipa::path(
    put,
    path = "/api/drafts/{deployment_id}/{config_file_id}",
    tag = "admin",
    params(
        ("deployment_id" = i64, Path, description = "Deployment instance ID"),
        ("config_file_id" = i64, Path, description = "Config file ID")
    ),
    request_body = crate::openapi::UpdateDraftRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Draft created or updated", body = DraftResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance or config file not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Draft version conflict", body = crate::error::ErrorResponse),
        (status = 422, description = "Draft validation failed", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn put_draft(
    State(state): State<AppState>,
    Path((deployment_id, config_file_id)): Path<(i64, i64)>,
    headers: HeaderMap,
    payload: Result<Json<UpdateDraftRequest>, JsonRejection>,
) -> Result<Json<DraftResponse>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await?;

    let context = load_draft_context(pool, deployment_id, config_file_id).await?;
    validate_draft_payload(&payload, &context)?;

    let row = if let Some(existing) = sqlx::query(
        r#"
        SELECT version
        FROM drafts
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    {
        let current_version: i64 = existing.get("version");
        if payload.base_version != Some(current_version) {
            return Err(ApiError::conflict(
                "draft_version_conflict",
                "draft version conflict",
            ));
        }

        sqlx::query(
            r#"
            UPDATE drafts
            SET
                content = $3,
                content_hash = $4,
                format = $5,
                schema_version = $6,
                version = version + 1,
                editor_user_id = $7,
                updated_at = NOW()
            WHERE deployment_instance_id = $1
              AND config_file_id = $2
            RETURNING
                deployment_instance_id,
                config_file_id,
                format,
                content,
                version,
                schema_version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            "#,
        )
        .bind(deployment_id)
        .bind(config_file_id)
        .bind(&payload.content)
        .bind(hash_content(&payload.content))
        .bind(&payload.format)
        .bind(context.schema_version)
        .bind(auth.user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal())?
    } else {
        if payload.base_version.is_some_and(|version| version != 0) {
            return Err(ApiError::conflict(
                "draft_version_conflict",
                "draft version conflict",
            ));
        }

        sqlx::query(
            r#"
            INSERT INTO drafts (
                project_id,
                config_file_id,
                deployment_instance_id,
                content,
                content_hash,
                format,
                schema_version,
                version,
                editor_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 1, $8)
            RETURNING
                deployment_instance_id,
                config_file_id,
                format,
                content,
                version,
                schema_version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            "#,
        )
        .bind(context.project_id)
        .bind(config_file_id)
        .bind(deployment_id)
        .bind(&payload.content)
        .bind(hash_content(&payload.content))
        .bind(&payload.format)
        .bind(context.schema_version)
        .bind(auth.user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal())?
    };

    Ok(Json(map_draft_row(row)))
}

impl UpdateDraftRequest {
    fn validate(self) -> Result<ValidatedUpdateDraftRequest, ApiError> {
        Ok(ValidatedUpdateDraftRequest {
            content: required_present(self.content, "content")?,
            format: required(self.format, "format")?,
            base_version: self.base_version,
        })
    }
}

async fn load_draft_context(
    pool: &sqlx::PgPool,
    deployment_id: i64,
    config_file_id: i64,
) -> Result<DraftContext, ApiError> {
    let deployment_project_id: i64 = sqlx::query_scalar(
        r#"
        SELECT project_id
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    let row = sqlx::query(
        r#"
        SELECT project_id, format, schema_version
        FROM config_files
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(config_file_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("config_file_not_found", "config file not found"))?;

    let config_project_id: i64 = row.get("project_id");
    if deployment_project_id != config_project_id {
        return Err(ApiError::not_found_with(
            "config_file_not_found",
            "config file not found",
        ));
    }

    Ok(DraftContext {
        project_id: config_project_id,
        format: row.get("format"),
        schema_version: row.get("schema_version"),
    })
}

fn map_draft_row(row: sqlx::postgres::PgRow) -> DraftResponse {
    DraftResponse {
        deployment_instance_id: row.get("deployment_instance_id"),
        config_file_id: row.get("config_file_id"),
        format: row.get("format"),
        content: row.get("content"),
        version: row.get("version"),
        schema_version: row.get("schema_version"),
        updated_at: row.get("updated_at"),
    }
}

fn validate_draft_payload(
    payload: &ValidatedUpdateDraftRequest,
    context: &DraftContext,
) -> Result<(), ApiError> {
    if payload.format != context.format {
        return Err(ApiError::unprocessable_entity(
            "draft_validation_failed",
            "draft format must match config file format",
        ));
    }

    Ok(())
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

fn required_present(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
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
        "content" => "invalid request body: content is required",
        "format" => "invalid request body: format is required",
        _ => "invalid request body",
    }
}

fn hash_content(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);

    for byte in digest {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }

    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("nibble should always be within 0..=15"),
    }
}

#[cfg(test)]
mod tests {
    use super::hash_content;

    #[test]
    fn hashes_draft_content_to_sha256_hex() {
        assert_eq!(
            hash_content("poll_interval_ms: 5000\n"),
            "aff9517cad8914cd20ffc758cc73cc77d61364393beb85ae464183be5fbffde8"
        );
    }
}
