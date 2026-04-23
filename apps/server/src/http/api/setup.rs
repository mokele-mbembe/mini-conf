use crate::{
    audit::{AuditLogEntry, write_audit_log},
    authorization::require_platform_admin,
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use schema::setup::SetupStatusResponse;
use sqlx::{PgPool, Row};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup/status", get(get_setup_status))
        .route("/setup/complete", post(complete_setup))
}

#[utoipa::path(
    get,
    path = "/api/setup/status",
    tag = "setup",
    responses(
        (status = 200, description = "Current setup status", body = SetupStatusResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_setup_status(
    State(state): State<AppState>,
) -> Result<Json<SetupStatusResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    Ok(Json(load_setup_status(pool).await?))
}

#[utoipa::path(
    post,
    path = "/api/setup/complete",
    tag = "setup",
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Setup marked as completed", body = SetupStatusResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 403, description = "Platform admin access required", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn complete_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SetupStatusResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = require_platform_admin(pool, &headers).await?;
    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;

    let result = sqlx::query(
        r#"
        WITH current AS (
            SELECT setup_completed_at IS NOT NULL AS already_completed
            FROM system_settings
            WHERE id = 1
        )
        UPDATE system_settings
        SET
            setup_completed_at = COALESCE(setup_completed_at, NOW()),
            setup_completed_by_user_id = COALESCE(setup_completed_by_user_id, $1),
            updated_at = NOW()
        WHERE id = 1
        RETURNING (SELECT already_completed FROM current) AS already_completed
        "#,
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::internal())?;

    let already_completed: bool = result.get("already_completed");
    if !already_completed {
        write_audit_log(
            &mut *tx,
            AuditLogEntry {
                project_id: None,
                user_id: Some(auth.user_id),
                action: "setup.completed",
                resource_type: "setup",
                resource_id: "system".to_owned(),
                detail: Some(serde_json::json!({
                    "completed_by_user_id": auth.user_id
                })),
            },
        )
        .await?;
    }

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(load_setup_status(pool).await?))
}

async fn load_setup_status(pool: &PgPool) -> Result<SetupStatusResponse, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            to_char(ss.setup_completed_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS setup_completed_at,
            ss.setup_completed_by_user_id,
            (
                SELECT COUNT(*)::bigint
                FROM users
                WHERE status = 'active' AND is_platform_admin = TRUE
            ) AS active_platform_admin_count,
            (
                SELECT COUNT(*)::bigint
                FROM projects
            ) AS project_count
        FROM system_settings ss
        WHERE ss.id = 1
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let response = if let Some(row) = row {
        let setup_completed_at: Option<String> = row.get("setup_completed_at");
        SetupStatusResponse {
            setup_required: setup_completed_at.is_none(),
            setup_completed_at,
            setup_completed_by_user_id: row.get("setup_completed_by_user_id"),
            active_platform_admin_count: row.get("active_platform_admin_count"),
            project_count: row.get("project_count"),
        }
    } else {
        SetupStatusResponse {
            setup_required: true,
            setup_completed_at: None,
            setup_completed_by_user_id: None,
            active_platform_admin_count: 0,
            project_count: 0,
        }
    };

    Ok(response)
}
