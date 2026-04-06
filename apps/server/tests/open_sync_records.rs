use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::open::DeploymentSyncResponse;
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;

const TEST_TOKEN: &str = "mini-conf-open-sync-token";

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping open sync record integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_open_sync_{nanos}")
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

async fn seed_release(pool: &PgPool) {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("project should insert");

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'main', 'Main', 'yaml') RETURNING id",
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

async fn seed_credential_only(pool: &PgPool) {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy-auth', 'Coffee Legacy Auth') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("project should insert");

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name) VALUES ($1, 'prod', 'store-auth', 'Store Auth') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .expect("deployment should insert");

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
async fn sync_record_inserts_row_and_returns_ok() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/deployment-sync-records")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-001",
                        "config":"main",
                        "process_key":"main",
                        "action":"apply",
                        "revision":"20260405.0001",
                        "status":"success",
                        "message":"config applied",
                        "detail":{"duration_ms":87},
                        "reported_at":"2026-04-05T12:05:00Z"
                    }"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentSyncResponse = read_json(response).await;
    assert_eq!(payload, DeploymentSyncResponse { ok: true });

    let row = sqlx::query(
        r#"
        SELECT action, status, process_key, revision, message, detail->>'duration_ms' AS duration_ms
        FROM deployment_sync_records
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("sync record should insert");

    assert_eq!(row.get::<String, _>("action"), "apply");
    assert_eq!(row.get::<String, _>("status"), "success");
    assert_eq!(row.get::<String, _>("process_key"), "main");
    assert_eq!(row.get::<String, _>("revision"), "20260405.0001");
    assert_eq!(row.get::<String, _>("message"), "config applied");
    assert_eq!(row.get::<String, _>("duration_ms"), "87");

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn sync_record_returns_not_found_for_unknown_deployment() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_credential_only(&pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/deployment-sync-records")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-missing",
                        "config":"main",
                        "action":"apply",
                        "status":"success"
                    }"#,
                ))
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "deployment_not_found".to_owned(),
            message: "deployment instance not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn sync_record_returns_not_found_for_unknown_release() {
    let Some((app, pool, database_url, schema)) = setup_app().await else {
        return;
    };

    seed_release(&pool).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/deployment-sync-records")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-001",
                        "config":"main",
                        "action":"apply",
                        "revision":"20260405.9999",
                        "status":"failed"
                    }"#,
                ))
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
