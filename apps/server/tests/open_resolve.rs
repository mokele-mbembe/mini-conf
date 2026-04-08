use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tower::util::ServiceExt;

const TEST_TOKEN: &str = "mini-conf-open-resolve-token";

async fn setup_app() -> Option<(axum::Router, PgPool, String, String)> {
    let database_url = test_database_url("open resolve")?;
    let schema = unique_schema_name("mini_conf_open_resolve");
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

async fn seed_release(pool: &PgPool, config_code: &str) {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("project should insert");

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, $2, 'Main', 'yaml') RETURNING id",
    )
    .bind(project_id)
    .bind(config_code)
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
        ) VALUES ($1, $2, $3, '20260405.0001', 'log_level: info\n', 'abc123', 'yaml', 'initial', 'soft', 1)",
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
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
async fn resolve_returns_release_payload_and_etag() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "main").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main")
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

    let payload: serde_json::Value = read_json(response).await;
    assert_eq!(payload["project"], "coffee-legacy");
    assert_eq!(payload["environment"], "prod");
    assert_eq!(payload["deployment"]["key"], "store-001");
    assert_eq!(payload["config"], "main");
    assert_eq!(payload["release"]["revision"], "20260405.0001");
    assert_eq!(payload["fetch"]["url"], "/api/open/releases/20260405.0001");

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn resolve_returns_unauthorized_when_bearer_token_is_missing() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "main").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(response).await;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "missing_token".to_owned(),
            message: "Missing Bearer token".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn resolve_returns_not_modified_when_client_already_has_latest_revision() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "main").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main&current_revision=20260405.0001")
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
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    assert!(body.is_empty());

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn resolve_returns_config_file_not_found_when_config_is_missing() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool, "main").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=vision")
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
            code: "config_file_not_found".to_owned(),
            message: "config file not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await;
}
