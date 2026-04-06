use crate::{auth::authenticate_admin_session, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    routing::post,
};
use schema::release::ReleaseSummary;
use serde::Deserialize;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct PublishReleaseRequest {
    project_id: Option<i64>,
    deployment_instance_id: Option<i64>,
    config_file_id: Option<i64>,
    change_summary: Option<String>,
}

#[derive(Debug)]
struct ValidatedPublishReleaseRequest {
    project_id: i64,
    deployment_instance_id: i64,
    config_file_id: i64,
    change_summary: Option<String>,
}

#[derive(Debug)]
struct ReleasePublishContext {
    format: String,
}

#[derive(Debug)]
struct DraftForPublish {
    content: String,
    content_hash: String,
    format: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/releases/publish", post(publish_release))
}

#[utoipa::path(
    post,
    path = "/api/releases/publish",
    tag = "admin",
    request_body = crate::openapi::PublishReleaseRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Release published from current draft", body = ReleaseSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project, deployment instance, config file, or draft not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn publish_release(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<PublishReleaseRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ReleaseSummary>), ApiError> {
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

    let context = load_publish_context(
        pool,
        payload.project_id,
        payload.deployment_instance_id,
        payload.config_file_id,
    )
    .await?;
    let draft =
        load_draft_for_publish(pool, payload.deployment_instance_id, payload.config_file_id)
            .await?;
    if draft.format != context.format {
        return Err(ApiError::unprocessable_entity(
            "release_publish_failed",
            "draft format no longer matches config file",
        ));
    }
    let revision = next_revision(pool).await?;

    let row = sqlx::query(
        r#"
        INSERT INTO releases (
            project_id,
            config_file_id,
            deployment_instance_id,
            revision,
            content,
            content_hash,
            format,
            change_summary,
            diff_summary,
            apply_mode,
            published_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, 'soft', $9)
        RETURNING
            id,
            project_id,
            deployment_instance_id,
            config_file_id,
            revision,
            btrim(content_hash) AS content_hash,
            format,
            change_summary,
            apply_mode,
            published_by,
            to_char(published_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS published_at
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.config_file_id)
    .bind(payload.deployment_instance_id)
    .bind(revision)
    .bind(draft.content)
    .bind(draft.content_hash)
    .bind(draft.format)
    .bind(payload.change_summary)
    .bind(auth.user_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(map_release_row(row))))
}

impl PublishReleaseRequest {
    fn validate(self) -> Result<ValidatedPublishReleaseRequest, ApiError> {
        Ok(ValidatedPublishReleaseRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            deployment_instance_id: required_i64(
                self.deployment_instance_id,
                "deployment_instance_id",
            )?,
            config_file_id: required_i64(self.config_file_id, "config_file_id")?,
            change_summary: normalize_optional(self.change_summary),
        })
    }
}

async fn load_publish_context(
    pool: &sqlx::PgPool,
    project_id: i64,
    deployment_instance_id: i64,
    config_file_id: i64,
) -> Result<ReleasePublishContext, ApiError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT id
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

    let deployment_project_id: i64 = sqlx::query_scalar(
        r#"
        SELECT project_id
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(deployment_instance_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    if deployment_project_id != project_id {
        return Err(ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        ));
    }

    let row = sqlx::query(
        r#"
        SELECT project_id, format
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

    if row.get::<i64, _>("project_id") != project_id {
        return Err(ApiError::not_found_with(
            "config_file_not_found",
            "config file not found",
        ));
    }

    Ok(ReleasePublishContext {
        format: row.get("format"),
    })
}

async fn load_draft_for_publish(
    pool: &sqlx::PgPool,
    deployment_instance_id: i64,
    config_file_id: i64,
) -> Result<DraftForPublish, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT content, btrim(content_hash) AS content_hash, format
        FROM drafts
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
        LIMIT 1
        "#,
    )
    .bind(deployment_instance_id)
    .bind(config_file_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| ApiError::not_found_with("draft_not_found", "draft not found"))?;

    Ok(DraftForPublish {
        content: row.get("content"),
        content_hash: row.get("content_hash"),
        format: row.get("format"),
    })
}

async fn next_revision(pool: &sqlx::PgPool) -> Result<String, ApiError> {
    sqlx::query_scalar(
        r#"
        SELECT
            to_char(NOW() AT TIME ZONE 'UTC', 'YYYYMMDD') || '.' ||
            lpad((COALESCE(MAX(right(revision, 4)::int), 0) + 1)::text, 4, '0') AS revision
        FROM releases
        WHERE revision LIKE to_char(NOW() AT TIME ZONE 'UTC', 'YYYYMMDD') || '.%'
        "#,
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())
}

fn map_release_row(row: sqlx::postgres::PgRow) -> ReleaseSummary {
    ReleaseSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        deployment_instance_id: row.get("deployment_instance_id"),
        config_file_id: row.get("config_file_id"),
        revision: row.get("revision"),
        content_hash: row.get("content_hash"),
        format: row.get("format"),
        change_summary: row.get("change_summary"),
        apply_mode: row.get("apply_mode"),
        published_by: row.get("published_by"),
        published_at: row.get("published_at"),
    }
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
        "project_id" => "invalid request body: project_id is required",
        "deployment_instance_id" => "invalid request body: deployment_instance_id is required",
        "config_file_id" => "invalid request body: config_file_id is required",
        _ => "invalid request body",
    }
}

#[cfg(test)]
mod tests {
    use super::invalid_body_message;

    #[test]
    fn invalid_body_message_covers_publish_fields() {
        assert_eq!(
            invalid_body_message("deployment_instance_id"),
            "invalid request body: deployment_instance_id is required"
        );
    }
}
