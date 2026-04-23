use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::open::DeploymentSyncResponse;
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use std::{error::Error, io};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const TEST_TOKEN: &str = "mini-conf-open-heartbeat-token";

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("open heartbeat") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_open_heartbeat");
    let mut admin = PgConnection::connect(&database_url).await?;

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let pool = state
        .db_pool()
        .ok_or_else(|| io::Error::other("db pool should be present after bootstrap"))?
        .clone();
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

async fn seed_deployment(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    sqlx::query("INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'vision', 'Vision', 'yaml')")
        .bind(project_id)
        .execute(pool)
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
async fn heartbeat_upserts_latest_config_state() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_deployment(&pool).await?;

    for body in [
        r#"{
            "project":"coffee-legacy",
            "environment":"prod",
            "deployment_key":"store-001",
            "config":"vision",
            "metadata":{"ip":"10.0.0.8","version":"1.0.3"},
            "reported_at":"2026-04-05T12:05:00Z"
        }"#,
        r#"{
            "project":"coffee-legacy",
            "environment":"prod",
            "deployment_key":"store-001",
            "config":"vision",
            "metadata":{"ip":"10.0.0.9","version":"1.0.4"},
            "reported_at":"2026-04-05T12:06:00Z"
        }"#,
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/open/heartbeats")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                    .body(Body::from(body))?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let payload: DeploymentSyncResponse = read_json(response).await?;
        assert_eq!(payload, DeploymentSyncResponse { ok: true });
    }

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) OVER() AS heartbeat_count,
            config_file_id,
            metadata->>'ip' AS ip,
            metadata->>'version' AS version,
            to_char(reported_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS reported_at
        FROM deployment_heartbeats
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(row.get::<i64, _>("heartbeat_count"), 1);
    assert!(row.get::<i64, _>("config_file_id") > 0);
    assert_eq!(row.get::<String, _>("ip"), "10.0.0.9");
    assert_eq!(row.get::<String, _>("version"), "1.0.4");
    assert_eq!(row.get::<String, _>("reported_at"), "2026-04-05T12:06:00Z");

    teardown(&database_url, &schema, pool).await?;
    Ok(())
}

#[tokio::test]
async fn heartbeat_returns_not_found_for_unknown_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_credential_only(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/heartbeats")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-missing",
                        "config":"vision"
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
async fn heartbeat_returns_not_found_for_inactive_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_deployment(&pool).await?;
    sqlx::query(
        "UPDATE deployment_instances SET status = 'inactive' WHERE deployment_key = 'store-001'",
    )
    .execute(&pool)
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/open/heartbeats")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::from(
                    r#"{
                        "project":"coffee-legacy",
                        "environment":"prod",
                        "deployment_key":"store-001",
                        "config":"vision"
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
