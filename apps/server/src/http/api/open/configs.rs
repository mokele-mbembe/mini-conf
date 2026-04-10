use crate::{
    auth::{authenticate_open_request, ensure_deployment_access},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use schema::open::{ResolveConfigResponse, ResolveDeployment, ResolveFetch, ResolveRelease};
use serde::Deserialize;
use sqlx::Row;

#[derive(Debug, Deserialize)]
pub(crate) struct ResolveConfigQuery {
    project: Option<String>,
    environment: Option<String>,
    deployment_key: Option<String>,
    config: Option<String>,
    process_key: Option<String>,
    current_revision: Option<String>,
}

#[derive(Debug)]
struct ValidatedResolveConfigQuery {
    project: String,
    environment: String,
    deployment_key: String,
    config: String,
    process_key: Option<String>,
    current_revision: Option<String>,
}

#[derive(Debug)]
struct DeploymentLookup {
    project_id: i64,
    deployment_id: i64,
    deployment_name: String,
}

#[derive(Debug)]
struct ReleaseLookup {
    revision: String,
    content_hash: String,
    format: String,
    published_at: String,
    apply_mode: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/open/configs/resolve", get(resolve_config))
}

#[utoipa::path(
    get,
    path = "/api/open/configs/resolve",
    tag = "open",
    params(crate::openapi::ResolveConfigParams),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Resolve latest release for one config", body = ResolveConfigResponse, headers(("etag" = String, description = "Entity tag for release content hash"))),
        (status = 304, description = "Client already has the latest release", headers(("etag" = String, description = "Entity tag for release content hash"))),
        (status = 400, description = "Invalid query parameters", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::error::ErrorResponse),
        (status = 403, description = "Bearer token does not match target deployment", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment, config file, or release not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn resolve_config(
    State(state): State<AppState>,
    Query(query): Query<ResolveConfigQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let query = query.validate()?;
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_open_request(pool, &headers).await?;

    let deployment = find_deployment(
        pool,
        &query.project,
        &query.environment,
        &query.deployment_key,
    )
    .await?
    .ok_or_else(|| {
        ApiError::not_found_with("deployment_not_found", "deployment instance not found")
    })?;
    ensure_deployment_access(auth, deployment.deployment_id)?;

    let config_file_id = find_config_file(pool, deployment.project_id, &query.config)
        .await?
        .ok_or_else(|| {
            ApiError::not_found_with("config_file_not_found", "config file not found")
        })?;

    let release = find_latest_release(pool, deployment.deployment_id, config_file_id)
        .await?
        .ok_or_else(|| ApiError::not_found_with("release_not_found", "release not found"))?;

    if release_is_not_modified(&query, &headers, &release) {
        return Ok(not_modified_response(&release.content_hash));
    }

    Ok(resolve_response(query, deployment, release))
}

impl ResolveConfigQuery {
    fn validate(self) -> Result<ValidatedResolveConfigQuery, ApiError> {
        Ok(ValidatedResolveConfigQuery {
            project: required(self.project, "project")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            config: required(self.config, "config")?,
            process_key: self.process_key,
            current_revision: self.current_revision,
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
        "deployment_key" => "missing required query parameter: deployment_key",
        "config" => "missing required query parameter: config",
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
          AND p.status = 'active'
          AND d.environment = $2
          AND d.deployment_key = $3
          AND d.status = 'active'
        "#,
    )
    .bind(project)
    .bind(environment)
    .bind(deployment_key)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| DeploymentLookup {
        project_id: row.get("project_id"),
        deployment_id: row.get("deployment_id"),
        deployment_name: row.get("deployment_name"),
    }))
}

async fn find_config_file(
    pool: &sqlx::PgPool,
    project_id: i64,
    config: &str,
) -> Result<Option<i64>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM config_files
        WHERE project_id = $1
          AND code = $2
          AND status = 'active'
        "#,
    )
    .bind(project_id)
    .bind(config)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| row.get("id")))
}

async fn find_latest_release(
    pool: &sqlx::PgPool,
    deployment_id: i64,
    config_file_id: i64,
) -> Result<Option<ReleaseLookup>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            revision,
            btrim(content_hash) AS content_hash,
            format,
            to_char(published_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS published_at,
            apply_mode
        FROM releases
        WHERE deployment_instance_id = $1
          AND config_file_id = $2
        ORDER BY published_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| ReleaseLookup {
        revision: row.get("revision"),
        content_hash: row.get("content_hash"),
        format: row.get("format"),
        published_at: row.get("published_at"),
        apply_mode: row.get("apply_mode"),
    }))
}

fn release_is_not_modified(
    query: &ValidatedResolveConfigQuery,
    headers: &HeaderMap,
    release: &ReleaseLookup,
) -> bool {
    if query.current_revision.as_deref() == Some(release.revision.as_str()) {
        return true;
    }

    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .map(normalize_etag)
        .is_some_and(|etag| etag == release.content_hash)
}

fn normalize_etag(value: &str) -> &str {
    value.trim().trim_matches('"')
}

fn etag_header(content_hash: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{content_hash}\""))
        .expect("content hashes should always produce valid ETag headers")
}

fn not_modified_response(content_hash: &str) -> Response {
    let mut response = StatusCode::NOT_MODIFIED.into_response();
    response
        .headers_mut()
        .insert(header::ETAG, etag_header(content_hash));
    response
}

fn resolve_response(
    query: ValidatedResolveConfigQuery,
    deployment: DeploymentLookup,
    release: ReleaseLookup,
) -> Response {
    let _ = &query.process_key;

    let body = ResolveConfigResponse {
        project: query.project,
        environment: query.environment,
        deployment: ResolveDeployment {
            key: query.deployment_key,
            name: deployment.deployment_name,
        },
        config: query.config,
        release: ResolveRelease {
            revision: release.revision.clone(),
            content_hash: release.content_hash.clone(),
            format: release.format,
            published_at: release.published_at,
            apply_mode: release.apply_mode,
        },
        fetch: ResolveFetch {
            url: format!("/api/open/releases/{}", release.revision),
        },
    };

    let mut response = Json(body).into_response();
    response
        .headers_mut()
        .insert(header::ETAG, etag_header(&release.content_hash));
    response
}
