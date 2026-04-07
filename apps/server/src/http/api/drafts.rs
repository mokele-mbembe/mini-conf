use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::{HeaderMap, header},
    routing::get,
};
use schema::draft::{DraftCloneResponse, DraftResponse};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateDraftRequest {
    content: Option<String>,
    format: Option<String>,
    base_version: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CloneDraftRequest {
    source_deployment_instance_id: Option<i64>,
    source_kind: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateDraftRequest {
    content: String,
    format: String,
    base_version: Option<i64>,
}

#[derive(Debug)]
struct ValidatedCloneDraftRequest {
    source_deployment_instance_id: i64,
    source_kind: String,
}

#[derive(Debug)]
struct DraftContext {
    project_id: i64,
    format: String,
    schema_version: Option<String>,
}

#[derive(Debug)]
struct DraftCloneSource {
    content: String,
    content_hash: String,
    format: String,
    schema_version: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/drafts/{deployment_id}/{config_file_id}",
            get(get_draft).put(put_draft),
        )
        .route(
            "/drafts/{target_deployment_id}/{config_file_id}/clone",
            axum::routing::post(clone_draft),
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

#[utoipa::path(
    post,
    path = "/api/drafts/{target_deployment_id}/{config_file_id}/clone",
    tag = "admin",
    params(
        ("target_deployment_id" = i64, Path, description = "Target deployment instance ID"),
        ("config_file_id" = i64, Path, description = "Config file ID")
    ),
    request_body = crate::openapi::CloneDraftRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Draft cloned from source deployment", body = DraftCloneResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Cross-project draft clone is forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance, config file, or source revision not found", body = crate::error::ErrorResponse),
        (status = 422, description = "Draft validation failed", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn clone_draft(
    State(state): State<AppState>,
    Path((target_deployment_id, config_file_id)): Path<(i64, i64)>,
    headers: HeaderMap,
    payload: Result<Json<CloneDraftRequest>, JsonRejection>,
) -> Result<Json<DraftCloneResponse>, ApiError> {
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

    let context = load_draft_context(pool, target_deployment_id, config_file_id).await?;
    ensure_same_project(
        pool,
        context.project_id,
        payload.source_deployment_instance_id,
        target_deployment_id,
    )
    .await?;

    let source = load_clone_source(
        pool,
        payload.source_deployment_instance_id,
        config_file_id,
        &payload.source_kind,
    )
    .await?;
    validate_cloned_draft(&context, &source)?;

    let row = upsert_draft(
        pool,
        target_deployment_id,
        config_file_id,
        &context,
        &source,
        auth.user_id,
    )
    .await?;

    Ok(Json(DraftCloneResponse {
        draft: map_draft_row(row),
        source_deployment_instance_id: payload.source_deployment_instance_id,
        source_kind: payload.source_kind,
    }))
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

impl CloneDraftRequest {
    fn validate(self) -> Result<ValidatedCloneDraftRequest, ApiError> {
        Ok(ValidatedCloneDraftRequest {
            source_deployment_instance_id: self.source_deployment_instance_id.ok_or_else(|| {
                ApiError::bad_request(
                    "invalid_request",
                    "invalid request body: source_deployment_instance_id is required",
                )
            })?,
            source_kind: validate_source_kind(self.source_kind)?,
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

async fn ensure_same_project(
    pool: &sqlx::PgPool,
    project_id: i64,
    source_deployment_id: i64,
    target_deployment_id: i64,
) -> Result<(), ApiError> {
    let source_project_id: i64 = sqlx::query_scalar(
        r#"
        SELECT project_id
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(source_deployment_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    if source_project_id != project_id {
        return Err(ApiError::forbidden(
            "draft_clone_cross_project_forbidden",
            "draft clone source must be in the same project",
        ));
    }

    if source_deployment_id == target_deployment_id {
        return Ok(());
    }

    Ok(())
}

async fn load_clone_source(
    pool: &sqlx::PgPool,
    source_deployment_id: i64,
    config_file_id: i64,
    source_kind: &str,
) -> Result<DraftCloneSource, ApiError> {
    match source_kind {
        "draft" => {
            let row = sqlx::query(
                r#"
                SELECT content, btrim(content_hash) AS content_hash, format, schema_version
                FROM drafts
                WHERE deployment_instance_id = $1
                  AND config_file_id = $2
                LIMIT 1
                "#,
            )
            .bind(source_deployment_id)
            .bind(config_file_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(|| ApiError::not_found_with("draft_not_found", "draft not found"))?;

            Ok(DraftCloneSource {
                content: row.get("content"),
                content_hash: row.get("content_hash"),
                format: row.get("format"),
                schema_version: row.get("schema_version"),
            })
        }
        "latest_release" => {
            let row = sqlx::query(
                r#"
                SELECT content, btrim(content_hash) AS content_hash, format, NULL::varchar AS schema_version
                FROM releases
                WHERE deployment_instance_id = $1
                  AND config_file_id = $2
                ORDER BY published_at DESC, id DESC
                LIMIT 1
                "#,
            )
            .bind(source_deployment_id)
            .bind(config_file_id)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal())?
            .ok_or_else(|| ApiError::not_found_with("release_not_found", "release not found"))?;

            Ok(DraftCloneSource {
                content: row.get("content"),
                content_hash: row.get("content_hash"),
                format: row.get("format"),
                schema_version: row.get("schema_version"),
            })
        }
        _ => unreachable!("validated source kind should only allow supported variants"),
    }
}

async fn upsert_draft(
    pool: &sqlx::PgPool,
    deployment_id: i64,
    config_file_id: i64,
    context: &DraftContext,
    source: &DraftCloneSource,
    editor_user_id: i64,
) -> Result<sqlx::postgres::PgRow, ApiError> {
    if let Some(existing_version) = sqlx::query_scalar::<_, i64>(
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
        sqlx::query(
            r#"
            UPDATE drafts
            SET
                content = $3,
                content_hash = $4,
                format = $5,
                schema_version = $6,
                version = $7,
                editor_user_id = $8,
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
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
            "#,
        )
        .bind(deployment_id)
        .bind(config_file_id)
        .bind(&source.content)
        .bind(&source.content_hash)
        .bind(&source.format)
        .bind(source.schema_version.clone().or_else(|| context.schema_version.clone()))
        .bind(existing_version + 1)
        .bind(editor_user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal())
    } else {
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
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS updated_at
            "#,
        )
        .bind(context.project_id)
        .bind(config_file_id)
        .bind(deployment_id)
        .bind(&source.content)
        .bind(&source.content_hash)
        .bind(&source.format)
        .bind(source.schema_version.clone().or_else(|| context.schema_version.clone()))
        .bind(editor_user_id)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::internal())
    }
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

fn validate_cloned_draft(
    context: &DraftContext,
    source: &DraftCloneSource,
) -> Result<(), ApiError> {
    if source.format != context.format {
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

fn validate_source_kind(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            "invalid request body: source_kind is required",
        ));
    };

    match value.as_str() {
        "draft" | "latest_release" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid draft clone source kind",
        )),
    }
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
