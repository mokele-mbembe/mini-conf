use crate::{
    auth::{authenticate_open_request, ensure_deployment_access},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::get,
};
use schema::open::{ConfigBundleItem, ConfigBundleResponse, ResolveDeployment};
use serde::Deserialize;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigBundleQuery {
    project: Option<String>,
    environment: Option<String>,
}

#[derive(Debug)]
struct ValidatedConfigBundleQuery {
    project: String,
    environment: String,
}

#[derive(Debug)]
struct DeploymentLookup {
    deployment_id: i64,
    deployment_name: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route(
        "/open/deployments/{deployment_key}/config-bundle",
        get(get_config_bundle),
    )
}

#[utoipa::path(
    get,
    path = "/api/open/deployments/{deployment_key}/config-bundle",
    tag = "open",
    params(
        ("deployment_key" = String, Path, description = "Deployment instance key"),
        crate::openapi::ConfigBundleParams
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Current config bundle for a deployment", body = ConfigBundleResponse),
        (status = 400, description = "Invalid path or query parameters", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::error::ErrorResponse),
        (status = 403, description = "Bearer token does not match target deployment", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_config_bundle(
    State(state): State<AppState>,
    Path(deployment_key): Path<String>,
    Query(query): Query<ConfigBundleQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let query = query.validate()?;

    if deployment_key.trim().is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            "missing required path parameter: deployment_key",
        ));
    }

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_open_request(pool, &headers).await?;

    let deployment = find_deployment(pool, &query.project, &query.environment, &deployment_key)
        .await?
        .ok_or_else(|| {
            ApiError::not_found_with("deployment_not_found", "deployment instance not found")
        })?;
    ensure_deployment_access(auth, deployment.deployment_id)?;

    let configs = find_current_bundle(pool, deployment.deployment_id)
        .await?
        .into_iter()
        .map(|row| ConfigBundleItem {
            config: row.get("config"),
            revision: row.get("revision"),
            content_hash: row.get("content_hash"),
            format: row.get("format"),
            content: row.get("content"),
        })
        .collect();

    Ok(Json(ConfigBundleResponse {
        project: query.project,
        environment: query.environment,
        deployment: ResolveDeployment {
            key: deployment_key,
            name: deployment.deployment_name,
        },
        configs,
    })
    .into_response())
}

impl ConfigBundleQuery {
    fn validate(self) -> Result<ValidatedConfigBundleQuery, ApiError> {
        Ok(ValidatedConfigBundleQuery {
            project: required(self.project, "project")?,
            environment: required(self.environment, "environment")?,
        })
    }
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = value else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_query_message(field),
        ));
    };

    if value.trim().is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_query_message(field),
        ));
    }

    Ok(value)
}

fn invalid_query_message(field: &'static str) -> &'static str {
    match field {
        "project" => "missing required query parameter: project",
        "environment" => "missing required query parameter: environment",
        _ => "missing required query parameter",
    }
}

async fn find_deployment(
    pool: &sqlx::PgPool,
    project: &str,
    environment: &str,
    deployment_key: &str,
) -> Result<Option<DeploymentLookup>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT p.id AS project_id, d.id AS deployment_id, d.name AS deployment_name
        FROM projects p
        JOIN deployment_instances d ON d.project_id = p.id
        WHERE p.code = $1
          AND d.environment = $2
          AND d.deployment_key = $3
        "#,
    )
    .bind(project)
    .bind(environment)
    .bind(deployment_key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| DeploymentLookup {
        deployment_id: row.get("deployment_id"),
        deployment_name: row.get("deployment_name"),
    }))
}

async fn find_current_bundle(
    pool: &sqlx::PgPool,
    deployment_id: i64,
) -> Result<Vec<sqlx::postgres::PgRow>, ApiError> {
    sqlx::query(
        r#"
        SELECT
            cf.code AS config,
            latest.revision,
            latest.content_hash,
            latest.format,
            latest.content
        FROM config_files cf
        JOIN LATERAL (
            SELECT
                r.revision,
                btrim(r.content_hash) AS content_hash,
                r.format,
                r.content
            FROM releases r
            WHERE r.deployment_instance_id = $1
              AND r.config_file_id = cf.id
            ORDER BY r.published_at DESC, r.id DESC
            LIMIT 1
        ) AS latest ON TRUE
        WHERE cf.project_id = (
            SELECT project_id
            FROM deployment_instances
            WHERE id = $1
        )
        ORDER BY cf.code ASC
        "#,
    )
    .bind(deployment_id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())
}
