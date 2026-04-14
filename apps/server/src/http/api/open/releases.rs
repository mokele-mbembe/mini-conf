use crate::{auth::authenticate_open_request, error::ApiError, state::AppState};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use schema::open::{
    ReleaseConfig, ReleaseContentResponse, ReleaseDeployment, ReleaseMetadata, ResolveRelease,
};
use sqlx::Row;

#[derive(Debug)]
struct ReleaseLookup {
    project: String,
    environment: String,
    deployment_key: String,
    config_name: String,
    revision: String,
    content_hash: String,
    format: String,
    published_at: String,
    apply_mode: String,
    content: String,
    change_summary: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/open/releases/{revision}", get(get_release))
}

#[utoipa::path(
    get,
    path = "/api/open/releases/{revision}",
    tag = "open",
    params(
        ("revision" = String, Path, description = "Release revision to fetch")
    ),
    security(
        ("bearer_auth" = [])
    ),
    responses(
        (status = 200, description = "Release content payload", body = ReleaseContentResponse, headers(
            ("etag" = String, description = "Entity tag for release content hash"),
            ("cache-control" = String, description = "Cache policy")
        )),
        (status = 304, description = "Release not modified", headers(
            ("etag" = String, description = "Entity tag for release content hash"),
            ("cache-control" = String, description = "Cache policy")
        )),
        (status = 400, description = "Invalid revision path parameter", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or invalid bearer token", body = crate::error::ErrorResponse),
        (status = 404, description = "Release not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_release(
    State(state): State<AppState>,
    Path(revision): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if revision.trim().is_empty() {
        return Err(ApiError::bad_request(
            "invalid_request",
            "missing required path parameter: revision",
        ));
    }

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };
    let auth = authenticate_open_request(pool, &headers).await?;

    let release = find_release(pool, auth.deployment_instance_id, &revision)
        .await?
        .ok_or_else(|| ApiError::not_found_with("release_not_found", "release not found"))?;

    if release_is_not_modified(&headers, &release) {
        return Ok(not_modified_response(&release.content_hash));
    }

    Ok(release_response(release))
}

async fn find_release(
    pool: &sqlx::PgPool,
    deployment_instance_id: i64,
    revision: &str,
) -> Result<Option<ReleaseLookup>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            p.code AS project,
            d.environment,
            d.deployment_key,
            cf.code AS config_name,
            r.revision,
            btrim(r.content_hash) AS content_hash,
            r.format,
            to_char(r.published_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS published_at,
            r.apply_mode,
            r.content,
            r.change_summary
        FROM releases r
        JOIN projects p ON p.id = r.project_id
        JOIN config_files cf ON cf.id = r.config_file_id
        JOIN deployment_instances d ON d.id = r.deployment_instance_id
        WHERE r.revision = $1
          AND r.deployment_instance_id = $2
          AND p.status = 'active'
          AND cf.status = 'active'
          AND d.status = 'active'
        ORDER BY r.id DESC
        LIMIT 1
        "#,
    )
    .bind(revision)
    .bind(deployment_instance_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.map(|row| ReleaseLookup {
        project: row.get("project"),
        environment: row.get("environment"),
        deployment_key: row.get("deployment_key"),
        config_name: row.get("config_name"),
        revision: row.get("revision"),
        content_hash: row.get("content_hash"),
        format: row.get("format"),
        published_at: row.get("published_at"),
        apply_mode: row.get("apply_mode"),
        content: row.get("content"),
        change_summary: row.get("change_summary"),
    }))
}

fn release_is_not_modified(headers: &HeaderMap, release: &ReleaseLookup) -> bool {
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
    let headers = response.headers_mut();
    headers.insert(header::ETAG, etag_header(content_hash));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

fn release_response(release: ReleaseLookup) -> Response {
    let mut response = Json(ReleaseContentResponse {
        release: ResolveRelease {
            revision: release.revision.clone(),
            content_hash: release.content_hash.clone(),
            format: release.format,
            published_at: release.published_at,
            apply_mode: release.apply_mode,
        },
        deployment: ReleaseDeployment {
            project: release.project,
            environment: release.environment,
            deployment_key: release.deployment_key,
        },
        config: ReleaseConfig {
            name: release.config_name,
        },
        content: release.content,
        metadata: ReleaseMetadata {
            change_summary: release.change_summary,
        },
    })
    .into_response();

    let headers = response.headers_mut();
    headers.insert(header::ETAG, etag_header(&release.content_hash));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}
