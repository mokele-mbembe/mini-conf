use crate::{
    authorization::{ProjectRole, authenticate_user, require_project_role},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
};
use schema::audit::{AuditLogListResponse, AuditLogSummary};
use serde::Deserialize;
use sqlx::{Row, types::Json as SqlxJson};

#[derive(Debug, Deserialize)]
pub(crate) struct ListAuditLogsQuery {
    project_id: Option<i64>,
    user_id: Option<i64>,
    action: Option<String>,
    resource_type: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/audit-logs", get(list_audit_logs))
}

#[utoipa::path(
    get,
    path = "/api/audit-logs",
    tag = "admin",
    params(crate::openapi::ListAuditLogsParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Audit log list visible to the current project admin", body = AuditLogListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Current member cannot view audit logs", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_audit_logs(
    State(state): State<AppState>,
    Query(query): Query<ListAuditLogsQuery>,
    headers: HeaderMap,
) -> Result<Json<AuditLogListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_user(pool, &headers).await?;

    let rows = if let Some(project_id) = query.project_id {
        require_project_role(
            pool,
            auth.user_id,
            project_id,
            ProjectRole::Admin,
            "project_not_found",
            "project not found",
        )
        .await?;

        sqlx::query(
            r#"
            SELECT
                id,
                project_id,
                user_id,
                action,
                resource_type,
                resource_id,
                detail,
                to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
            FROM audit_logs
            WHERE project_id = $1
              AND ($2::bigint IS NULL OR user_id = $2)
              AND ($3::varchar IS NULL OR action = $3)
              AND ($4::varchar IS NULL OR resource_type = $4)
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(project_id)
        .bind(query.user_id)
        .bind(normalize_optional(query.action))
        .bind(normalize_optional(query.resource_type))
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal())?
    } else {
        sqlx::query(
            r#"
            SELECT
                al.id,
                al.project_id,
                al.user_id,
                al.action,
                al.resource_type,
                al.resource_id,
                al.detail,
                to_char(al.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS created_at
            FROM audit_logs al
            WHERE ($1::bigint IS NULL OR al.user_id = $1)
              AND ($2::varchar IS NULL OR al.action = $2)
              AND ($3::varchar IS NULL OR al.resource_type = $3)
              AND (
                    (al.project_id IS NOT NULL AND EXISTS (
                        SELECT 1
                        FROM project_members pm
                        WHERE pm.project_id = al.project_id
                          AND pm.user_id = $4
                          AND pm.role = 'admin'
                    ))
                      OR (al.project_id IS NULL AND ($5::boolean = TRUE OR al.user_id = $4))
              )
            ORDER BY al.created_at DESC, al.id DESC
            "#,
        )
        .bind(query.user_id)
        .bind(normalize_optional(query.action))
        .bind(normalize_optional(query.resource_type))
        .bind(auth.user_id)
              .bind(auth.is_platform_admin)
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::internal())?
    };

    Ok(Json(AuditLogListResponse {
        items: rows.iter().map(map_audit_log_row).collect(),
    }))
}

fn map_audit_log_row(row: &sqlx::postgres::PgRow) -> AuditLogSummary {
    AuditLogSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        user_id: row.get("user_id"),
        action: row.get("action"),
        resource_type: row.get("resource_type"),
        resource_id: row.get("resource_id"),
        detail: row
            .get::<Option<SqlxJson<serde_json::Value>>, _>("detail")
            .map(|value| value.0),
        created_at: row.get("created_at"),
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
