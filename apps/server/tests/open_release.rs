use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::open::ReleaseContentResponse;
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;

const TEST_TOKEN: &str = "mini-conf-open-release-token";

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping open release integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_open_release_{nanos}")
}

fn with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options[search_path]={schema}")
}

async fn setup_app() -> Option<(axum::Router, PgPool, String, String)> {
    let database_url = test_database_url()?;
    let schema = unique_schema_name();
    let mut admin = PgConnection::connect(&database_url)
        .await
        .expect("admin connection should succeed");

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("schema should be created");

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await
    .expect("state should build");
    let pool = state
        .db_pool()
        .expect("db pool should be present after bootstrap")
        .clone();
    let app = server::app(state);

    Some((app, pool, database_url, schema))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url)
        .await
        .expect("admin connection should succeed");
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await
        .expect("schema should be dropped");
}

async fn seed_release(pool: &PgPool, revision: &str) {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("project should insert");

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format, schema_version) VALUES ($1, 'main', 'Main', 'yaml', 'v1') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .expect("config file should insert");

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name) VALUES ($1, 'prod', 'store-001', 'Store 001') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .expect("deployment should insert");

    sqlx::query(
        "INSERT INTO releases (
            project_id,
            config_file_id,
            deployment_instance_id,
            revision,
            content,
            content_hash,
            format,
            change_summary,
            apply_mode,
            published_by
        ) VALUES ($1, $2, $3, $4, 'log_level: info\npoll_interval_sec: 30\n', 'abc123', 'yaml', 'adjust polling interval', 'soft', 1)",
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(revision)
    .execute(pool)
    .await
    .expect("release should insert");

    sqlx::query(
        "INSERT INTO deployment_credentials (deployment_instance_id, credential_name, token_hash) VALUES ($1, 'default', $2)",
    )
    .bind(deployment_id)
    .bind(server::auth::hash_bearer_token(TEST_TOKEN))
    .execute(pool)
    .await
    .expect("credential should insert");
}

async fn read_json<T: serde::de::DeserializeOwned>(response: axum::response::Response) -> T {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");

    serde_json::from_slice(&body).expect("body should be valid json")
}

#[tokio::test]
async fn release_returns_payload_and_cache_headers() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "20260405.0001").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/releases/20260405.0001")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::ETAG),
        Some(&header::HeaderValue::from_static("\"abc123\""))
    );
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&header::HeaderValue::from_static("no-cache"))
    );

    let payload: ReleaseContentResponse = read_json(response).await;
    assert_eq!(payload.release.revision, "20260405.0001");
    assert_eq!(payload.release.content_hash, "abc123");
    assert_eq!(payload.deployment.project, "coffee-legacy");
    assert_eq!(payload.deployment.environment, "prod");
    assert_eq!(payload.deployment.deployment_key, "store-001");
    assert_eq!(payload.config.name, "main");
    assert_eq!(payload.content, "log_level: info\npoll_interval_sec: 30\n");
    assert_eq!(payload.metadata.schema_version.as_deref(), Some("v1"));
    assert_eq!(
        payload.metadata.change_summary.as_deref(),
        Some("adjust polling interval")
    );

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn release_returns_not_modified_when_etag_matches() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "20260405.0001").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/releases/20260405.0001")
                .header(header::IF_NONE_MATCH, "\"abc123\"")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        response.headers().get(header::ETAG),
        Some(&header::HeaderValue::from_static("\"abc123\""))
    );
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL),
        Some(&header::HeaderValue::from_static("no-cache"))
    );

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    assert!(body.is_empty());

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn release_returns_not_found_for_unknown_revision() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "20260405.0001").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/releases/20260405.9999")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "release_not_found".to_owned(),
            message: "release not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await;
}
