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
use schema::draft::DraftResponse;
use schema::saved_version::{
    SavedVersionDetail, SavedVersionDetailResponse, SavedVersionListResponse,
    SavedVersionRestoreResponse, SavedVersionSummary,
};
use serde::Deserialize;
use sqlx::Row;

const NOTE_MAX_LENGTH: usize = 500;

#[derive(Debug, Deserialize)]
pub(crate) struct ListSavedVersionsQuery {
    deployment_instance_id: Option<i64>,
    config_file_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateSavedVersionRequest {
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RestoreSavedVersionRequest {
    base_version: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/draft-saved-versions", get(list_saved_versions))
        .route(
            "/draft-saved-versions/{id}",
            get(get_saved_version)
                .patch(update_saved_version)
                .delete(delete_saved_version),
        )
        .route(
            "/draft-saved-versions/{id}/restore",
            axum::routing::post(restore_saved_version),
        )
}

#[utoipa::path(
    get,
    path = "/api/draft-saved-versions",
    tag = "admin",
    params(crate::openapi::ListSavedVersionsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List saved versions", body = SavedVersionListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_saved_versions(
    State(state): State<AppState>,
    Query(query): Query<ListSavedVersionsQuery>,
    headers: HeaderMap,
) -> Result<Json<SavedVersionListResponse>, ApiError> {
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
            sv.id,
            sv.project_id,
            sv.deployment_instance_id,
            sv.config_file_id,
            sv.title,
            sv.note,
            sv.format,
            sv.source_draft_version,
            sv.created_by,
            u.username AS created_by_username,
            to_char(sv.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM draft_saved_versions sv
        JOIN users u ON u.id = sv.created_by
        JOIN project_members pm
          ON pm.project_id = sv.project_id
         AND pm.user_id = $1
         AND pm.role IN ('admin', 'editor')
        WHERE sv.deleted_at IS NULL
          AND ($2::bigint IS NULL OR sv.deployment_instance_id = $2)
          AND ($3::bigint IS NULL OR sv.config_file_id = $3)
        ORDER BY sv.created_at DESC, sv.id DESC
        "#,
    )
    .bind(auth.user_id)
    .bind(query.deployment_instance_id)
    .bind(query.config_file_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(SavedVersionListResponse {
        items: rows.iter().map(map_summary_row).collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/draft-saved-versions/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Saved version ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Saved version detail", body = SavedVersionDetailResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Saved version not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_saved_version(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<SavedVersionDetailResponse>, ApiError> {
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
            sv.id,
            sv.project_id,
            sv.deployment_instance_id,
            sv.config_file_id,
            sv.title,
            sv.note,
            sv.content,
            sv.content_hash,
            sv.format,
            sv.source_draft_version,
            sv.created_by,
            u.username AS created_by_username,
            to_char(sv.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM draft_saved_versions sv
        JOIN users u ON u.id = sv.created_by
        WHERE sv.id = $1
          AND sv.deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with("saved_version_not_found", "saved version not found")
    })?;

    let project_id: i64 = row.get("project_id");
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Editor,
        "saved_version_not_found",
        "saved version not found",
    )
    .await?;

    Ok(Json(SavedVersionDetailResponse {
        saved_version: map_detail_row(row),
    }))
}

#[utoipa::path(
    patch,
    path = "/api/draft-saved-versions/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Saved version ID")
    ),
    request_body = crate::openapi::UpdateSavedVersionRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Saved version updated", body = SavedVersionDetailResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Saved version not found", body = crate::error::ErrorResponse),
        (status = 422, description = "Note too long", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_saved_version(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateSavedVersionRequest>, JsonRejection>,
) -> Result<Json<SavedVersionDetailResponse>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;

    let note = payload
        .note
        .map(|n| n.trim().to_owned())
        .filter(|n| !n.is_empty());
    if let Some(ref n) = note {
        if n.len() > NOTE_MAX_LENGTH {
            return Err(ApiError::unprocessable_entity(
                "saved_version_note_too_long",
                "saved version note is too long",
            ));
        }
    }

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let existing = sqlx::query(
        r#"
        SELECT project_id, deployment_instance_id
        FROM draft_saved_versions
        WHERE id = $1
          AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with("saved_version_not_found", "saved version not found")
    })?;

    let project_id: i64 = existing.get("project_id");
    let deployment_instance_id: i64 = existing.get("deployment_instance_id");
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Editor,
        "saved_version_not_found",
        "saved version not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    sqlx::query(
        r#"
        UPDATE draft_saved_versions
        SET note = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(&note)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "saved_version.updated",
            resource_type: "saved_version",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": deployment_instance_id,
                "saved_version_id": id,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    let row = sqlx::query(
        r#"
        SELECT
            sv.id,
            sv.project_id,
            sv.deployment_instance_id,
            sv.config_file_id,
            sv.title,
            sv.note,
            sv.content,
            sv.content_hash,
            sv.format,
            sv.source_draft_version,
            sv.created_by,
            u.username AS created_by_username,
            to_char(sv.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
        FROM draft_saved_versions sv
        JOIN users u ON u.id = sv.created_by
        WHERE sv.id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(SavedVersionDetailResponse {
        saved_version: map_detail_row(row),
    }))
}

#[utoipa::path(
    post,
    path = "/api/draft-saved-versions/{id}/restore",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Saved version ID")
    ),
    request_body = crate::openapi::RestoreSavedVersionRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Current Draft restored from saved version", body = SavedVersionRestoreResponse),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Saved version not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Draft version conflict", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn restore_saved_version(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<RestoreSavedVersionRequest>, JsonRejection>,
) -> Result<Json<SavedVersionRestoreResponse>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let sv = sqlx::query(
        r#"
        SELECT
            project_id,
            deployment_instance_id,
            config_file_id,
            content,
            content_hash,
            format
        FROM draft_saved_versions
        WHERE id = $1
          AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with("saved_version_not_found", "saved version not found")
    })?;

    let project_id: i64 = sv.get("project_id");
    let deployment_instance_id: i64 = sv.get("deployment_instance_id");
    let config_file_id: i64 = sv.get("config_file_id");
    let content: String = sv.get("content");
    let content_hash: String = sv.get("content_hash");
    let format: String = sv.get("format");

    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Editor,
        "saved_version_not_found",
        "saved version not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    let draft_row = if let Some(existing) = sqlx::query(
        r#"
        SELECT version
        FROM drafts
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
        LIMIT 1
        "#,
    )
    .bind(deployment_instance_id)
    .bind(config_file_id)
    .fetch_optional(&mut *tx)
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
                version = version + 1,
                editor_user_id = $6,
                updated_at = NOW()
            WHERE deployment_instance_id = $1
              AND config_file_id = $2
            RETURNING
                deployment_instance_id,
                config_file_id,
                format,
                content,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            "#,
        )
        .bind(deployment_instance_id)
        .bind(config_file_id)
        .bind(&content)
        .bind(&content_hash)
        .bind(&format)
        .bind(auth.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?
    } else {
        if payload.base_version.is_some_and(|v| v != 0) {
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
                version,
                editor_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, 1, $7)
            RETURNING
                deployment_instance_id,
                config_file_id,
                format,
                content,
                version,
                to_char(updated_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS updated_at
            "#,
        )
        .bind(project_id)
        .bind(config_file_id)
        .bind(deployment_instance_id)
        .bind(&content)
        .bind(&content_hash)
        .bind(&format)
        .bind(auth.user_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| ApiError::internal())?
    };

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "saved_version.restored",
            resource_type: "saved_version",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": deployment_instance_id,
                "config_file_id": config_file_id,
                "saved_version_id": id,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(SavedVersionRestoreResponse {
        draft: DraftResponse {
            deployment_instance_id: draft_row.get("deployment_instance_id"),
            config_file_id: draft_row.get("config_file_id"),
            format: draft_row.get("format"),
            content: draft_row.get("content"),
            version: draft_row.get("version"),
            updated_at: draft_row.get("updated_at"),
        },
    }))
}

#[utoipa::path(
    delete,
    path = "/api/draft-saved-versions/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Saved version ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Saved version deleted"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Saved version not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_saved_version(
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

    let existing = sqlx::query(
        r#"
        SELECT project_id, deployment_instance_id, config_file_id
        FROM draft_saved_versions
        WHERE id = $1
          AND deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with("saved_version_not_found", "saved version not found")
    })?;

    let project_id: i64 = existing.get("project_id");
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Editor,
        "saved_version_not_found",
        "saved version not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    sqlx::query(
        r#"
        UPDATE draft_saved_versions
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "saved_version.deleted",
            resource_type: "saved_version",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": existing.get::<i64, _>("deployment_instance_id"),
                "config_file_id": existing.get::<i64, _>("config_file_id"),
                "saved_version_id": id,
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(StatusCode::NO_CONTENT)
}

fn map_summary_row(row: &sqlx::postgres::PgRow) -> SavedVersionSummary {
    SavedVersionSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        deployment_instance_id: row.get("deployment_instance_id"),
        config_file_id: row.get("config_file_id"),
        title: row.get("title"),
        note: row.get("note"),
        format: row.get("format"),
        source_draft_version: row.get("source_draft_version"),
        created_by: row.get("created_by"),
        created_by_username: row.get("created_by_username"),
        created_at: row.get("created_at"),
    }
}

fn map_detail_row(row: sqlx::postgres::PgRow) -> SavedVersionDetail {
    SavedVersionDetail {
        id: row.get("id"),
        project_id: row.get("project_id"),
        deployment_instance_id: row.get("deployment_instance_id"),
        config_file_id: row.get("config_file_id"),
        title: row.get("title"),
        note: row.get("note"),
        content: row.get("content"),
        content_hash: row.get("content_hash"),
        format: row.get("format"),
        source_draft_version: row.get("source_draft_version"),
        created_by: row.get("created_by"),
        created_by_username: row.get("created_by_username"),
        created_at: row.get("created_at"),
    }
}
