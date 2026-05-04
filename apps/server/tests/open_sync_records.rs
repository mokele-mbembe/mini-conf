use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::open::DeploymentSyncResponse;
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use tower::util::ServiceExt;

#[path = "support/mod.rs"]
mod support;

const TEST_TOKEN: &str = "mini-conf-open-sync-token";

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("open sync record") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_open_sync");
    let mut admin = PgConnection::connect(&database_url).await?;

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let pool = state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;
    sqlx::query(
        r#"
        UPDATE system_settings
        SET
            setup_completed_at = COALESCE(setup_completed_at, NOW()),
            updated_at = NOW()
        WHERE id = 1
        "#,
    )
    .execute(&pool)
    .await?;
    let app = server::app(state);

    Ok(Some((app, pool, database_url, schema)))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) -> TestResult {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url).await?;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;

    Ok(())
}

async fn seed_release(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'main', 'Main', 'yaml') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let environment_id: i64 = sqlx::query_scalar(
        "INSERT INTO project_environments (project_id, code, name, status, sort_order) VALUES ($1, 'prod', 'Production', 'active', 10) RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name) VALUES ($1, $2, 'store-001', 'Store 001') RETURNING id",
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await?;

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
    .await?;

    sqlx::query(
        "INSERT INTO deployment_credentials (deployment_instance_id, credential_name, token_hash) VALUES ($1, 'default', $2)",
    )
    .bind(deployment_id)
    .bind(server::auth::hash_bearer_token(TEST_TOKEN))
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_credential_only(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy-auth', 'Coffee Legacy Auth') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let environment_id: i64 = sqlx::query_scalar(
        "INSERT INTO project_environments (project_id, code, name, status, sort_order) VALUES ($1, 'prod', 'Production', 'active', 10) RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name) VALUES ($1, $2, 'store-auth', 'Store Auth') RETURNING id",
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await?;

    sqlx::query(
        "INSERT INTO deployment_credentials (deployment_instance_id, credential_name, token_hash) VALUES ($1, 'default', $2)",
    )
    .bind(deployment_id)
    .bind(server::auth::hash_bearer_token(TEST_TOKEN))
    .execute(pool)
    .await?;

    Ok(())
}

async fn read_json<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> TestResult<T> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;

    Ok(serde_json::from_slice(&body)?)
}

#[tokio::test]
async fn sync_record_inserts_row_and_returns_ok() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_release(&pool).await?;

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
                        "revision":"20260405.0001",
                        "status":"success",
                        "message":"config applied",
                        "detail":{"duration_ms":87},
                        "reported_at":"2026-04-05T12:05:00Z"
                    }"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentSyncResponse = read_json(response).await?;
    assert_eq!(payload, DeploymentSyncResponse { ok: true });

    let row = sqlx::query(
        r#"
        SELECT dsr.action, dsr.status, dsr.config_file_id, cf.code AS config, dsr.revision, dsr.message, dsr.detail->>'duration_ms' AS duration_ms
        FROM deployment_sync_records dsr
        JOIN config_files cf ON cf.id = dsr.config_file_id
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.get::<String, _>("action"), "apply");
    assert_eq!(row.get::<String, _>("status"), "success");
    assert_eq!(row.get::<String, _>("config"), "main");
    assert_eq!(row.get::<String, _>("revision"), "20260405.0001");
    assert_eq!(row.get::<String, _>("message"), "config applied");
    assert_eq!(row.get::<String, _>("duration_ms"), "87");

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}

#[tokio::test]
async fn sync_record_ignores_browser_csrf_cookies_when_using_bearer_auth() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_release(&pool).await?;

    let csrf_cookie = support::fetch_csrf_cookie(&app).await?;
    let session_cookie = support::login_as(&app, "admin", "admin123456").await?;
    let cookie_header = format!("{session_cookie}; {csrf_cookie}");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/deployment-sync-records")
                .header(header::COOKIE, cookie_header)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-001",
                        "config":"main",
                        "action":"apply",
                        "revision":"20260405.0001",
                        "status":"success",
                        "message":"config applied from browser context",
                        "detail":{"duration_ms":91},
                        "reported_at":"2026-04-05T12:05:00Z"
                    }"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentSyncResponse = read_json(response).await?;
    assert_eq!(payload, DeploymentSyncResponse { ok: true });

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}

#[tokio::test]
async fn sync_record_returns_not_found_for_unknown_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_credential_only(&pool).await?;

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
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "deployment_not_found".to_owned(),
            message: "deployment instance not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}

#[tokio::test]
async fn sync_record_returns_not_found_for_unknown_release() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_release(&pool).await?;

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
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "release_not_found".to_owned(),
            message: "release not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}

#[tokio::test]
async fn sync_record_returns_not_found_for_archived_config() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_release(&pool).await?;
    sqlx::query("UPDATE config_files SET status = 'archived' WHERE code = 'main'")
        .execute(&pool)
        .await?;

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
                        "status":"success"
                    }"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "config_file_not_found".to_owned(),
            message: "config file not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}
