use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::open::ConfigBundleResponse;
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{error::Error, io};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const TEST_TOKEN: &str = "mini-conf-open-bundle-token";

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("open config bundle") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_open_bundle");
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

async fn seed_bundle(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let main_config_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'main', 'Main', 'yaml') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let vision_config_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'vision', 'Vision', 'yaml') RETURNING id",
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
            published_by,
            published_at
        ) VALUES
            ($1, $2, $4, '20260405.0001', 'log_level: info\n', 'aaa', 'yaml', 'initial', 'soft', 1, '2026-04-05T12:00:00Z'),
            ($1, $2, $4, '20260405.0002', 'log_level: warn\n', 'bbb', 'yaml', 'raise level', 'soft', 1, '2026-04-05T12:05:00Z'),
            ($1, $3, $4, '20260405.0003', 'camera_enabled: true\n', 'ccc', 'yaml', 'enable vision', 'soft', 1, '2026-04-05T12:10:00Z')",
    )
    .bind(project_id)
    .bind(main_config_id)
    .bind(vision_config_id)
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

async fn seed_toml_bundle(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-toml', 'Coffee TOML') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'main', 'Main', 'toml') RETURNING id",
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
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name) VALUES ($1, $2, 'store-toml', 'Store TOML') RETURNING id",
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
            published_by,
            published_at
        ) VALUES
            ($1, $2, $3, '20260405.1001', 'log_level = \"warn\"\npoll_interval_sec = 45\n', 'toml-hash', 'toml', 'toml release', 'soft', 1, '2026-04-05T12:15:00Z')",
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

async fn seed_deployment_without_releases(pool: &PgPool) -> TestResult {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('empty-project', 'Empty Project') RETURNING id",
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
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name) VALUES ($1, $2, 'store-empty', 'Store Empty') RETURNING id",
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
async fn config_bundle_returns_latest_release_per_config() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_bundle(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/deployments/store-001/config-bundle?project=coffee-legacy&environment=prod")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let payload: ConfigBundleResponse = read_json(response).await?;
    assert_eq!(payload.project, "coffee-legacy");
    assert_eq!(payload.environment, "prod");
    assert_eq!(payload.deployment.key, "store-001");
    assert_eq!(payload.deployment.name, "Store 001");
    assert_eq!(payload.configs.len(), 2);
    assert_eq!(payload.configs[0].config, "main");
    assert_eq!(payload.configs[0].revision, "20260405.0002");
    assert_eq!(payload.configs[0].content_hash, "bbb");
    assert_eq!(payload.configs[0].content, "log_level: warn\n");
    assert_eq!(payload.configs[1].config, "vision");
    assert_eq!(payload.configs[1].revision, "20260405.0003");
    assert_eq!(payload.configs[1].content_hash, "ccc");

    teardown(&database_url, &schema, pool).await?;
    Ok(())
}

#[tokio::test]
async fn config_bundle_returns_empty_list_for_deployment_without_releases() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_deployment_without_releases(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/deployments/store-empty/config-bundle?project=empty-project&environment=prod")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let payload: ConfigBundleResponse = read_json(response).await?;
    assert!(payload.configs.is_empty());

    teardown(&database_url, &schema, pool).await?;
    Ok(())
}

#[tokio::test]
async fn config_bundle_returns_not_found_for_unknown_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_deployment_without_releases(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/deployments/store-missing/config-bundle?project=coffee-legacy&environment=prod")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())?,
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
async fn config_bundle_returns_not_found_for_inactive_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_bundle(&pool).await?;
    sqlx::query(
        "UPDATE deployment_instances SET status = 'inactive' WHERE deployment_key = 'store-001'",
    )
    .execute(&pool)
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/deployments/store-001/config-bundle?project=coffee-legacy&environment=prod")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())?,
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
async fn config_bundle_returns_toml_release_content() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    seed_toml_bundle(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/open/deployments/store-toml/config-bundle?project=coffee-toml&environment=prod")
                .header(header::AUTHORIZATION, format!("Bearer {TEST_TOKEN}"))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ConfigBundleResponse = read_json(response).await?;
    assert_eq!(payload.configs.len(), 1);
    assert_eq!(payload.configs[0].format, "toml");
    assert_eq!(
        payload.configs[0].content,
        "log_level = \"warn\"\npoll_interval_sec = 45\n"
    );

    teardown(&database_url, &schema, pool).await?;
    Ok(())
}
