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
use schema::clone_source::{CloneSourceAvailability, CloneSourceListResponse, CloneSourceSummary};
use serde::Deserialize;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct ListCloneSourcesQuery {
    project_id: i64,
    target_deployment_id: i64,
    config_file_id: i64,
    keyword: Option<String>,
    limit: Option<i64>,
    cursor: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/clone-sources", get(list_clone_sources))
}

fn validate_limit(limit: Option<i64>) -> Result<i64, ApiError> {
    let limit = limit.unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(ApiError::bad_request(
            "invalid_request",
            "limit must be between 1 and 50",
        ));
    }
    Ok(limit)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

#[utoipa::path(
    get,
    path = "/api/clone-sources",
    tag = "admin",
    params(crate::openapi::ListCloneSourcesParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List clone sources with availability", body = CloneSourceListResponse),
        (status = 400, description = "Invalid request parameters", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found or no access", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_clone_sources(
    State(state): State<AppState>,
    Query(query): Query<ListCloneSourcesQuery>,
    headers: HeaderMap,
) -> Result<Json<CloneSourceListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    // Clone sources is part of the editing workflow; require Editor or above
    require_project_role(
        pool,
        auth.user_id,
        query.project_id,
        ProjectRole::Editor,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;

    let limit = validate_limit(query.limit)?;
    let keyword = normalize_optional(query.keyword);
    let cursor = query.cursor.unwrap_or(0);

    // Fetch limit+1 rows to determine next_cursor
    let rows = sqlx::query(
        r#"
        SELECT
            di.id AS deployment_instance_id,
            di.deployment_key,
            di.name,
            di.environment_id,
            pe.name AS environment_name,
            di.is_template,
            EXISTS(
                SELECT 1
                FROM drafts d
                WHERE d.deployment_instance_id = di.id
                  AND d.config_file_id = $4
            ) AS has_draft,
            EXISTS(
                SELECT 1
                FROM releases r
                WHERE r.deployment_instance_id = di.id
                  AND r.config_file_id = $4
            ) AS has_latest_release
        FROM deployment_instances di
        JOIN project_environments pe
          ON pe.project_id = di.project_id
         AND pe.id = di.environment_id
        WHERE di.project_id = $1
          AND di.id != $2
          AND di.id > $3
          AND (
                $5::varchar IS NULL
                OR di.deployment_key ILIKE '%' || $5 || '%'
                OR di.name ILIKE '%' || $5 || '%'
          )
        ORDER BY di.id ASC
        LIMIT $6
        "#,
    )
    .bind(query.project_id)
    .bind(query.target_deployment_id)
    .bind(cursor)
    .bind(query.config_file_id)
    .bind(&keyword)
    .bind(limit + 1)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let has_more = rows.len() as i64 > limit;
    let items: Vec<CloneSourceSummary> = rows
        .into_iter()
        .take(limit as usize)
        .map(|row| CloneSourceSummary {
            deployment_instance_id: row.get("deployment_instance_id"),
            deployment_key: row.get("deployment_key"),
            name: row.get("name"),
            environment_id: row.get("environment_id"),
            environment_name: row.get("environment_name"),
            is_template: row.get("is_template"),
            available_sources: CloneSourceAvailability {
                draft: row.get("has_draft"),
                latest_release: row.get("has_latest_release"),
            },
        })
        .collect();

    let next_cursor = if has_more {
        items.last().map(|item| item.deployment_instance_id)
    } else {
        None
    };

    Ok(Json(CloneSourceListResponse { items, next_cursor }))
}
