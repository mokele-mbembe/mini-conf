use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    auth::AuthSessionResponse, deployment_instance::DeploymentTokenResetResponse,
    open::ResolveConfigResponse,
};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OLD_TOKEN: &str = "mini-conf-old-deployment-token";

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("deployment tokens") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_deployment_tokens");
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

    let Some(pool) = state.db_pool().cloned() else {
        return Err("db pool should be present after bootstrap".into());
    };

    Ok(Some((server::app(state), pool, database_url, schema)))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) -> TestResult {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url).await?;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;
    Ok(())
}

async fn read_json<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> TestResult<T> {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn session_cookie(response: &axum::response::Response) -> TestResult<String> {
    let Some(header_value) = response.headers().get(header::SET_COOKIE) else {
        return Err("set-cookie header should exist".into());
    };
    let value = header_value.to_str()?;
    let Some(cookie) = value.split(';').next() else {
        return Err("set-cookie should contain a session cookie".into());
    };
    Ok(cookie.to_owned())
}

async fn login(app: &axum::Router) -> TestResult<String> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"admin123456"}"#,
                ))?,
        )
        .await?;

    let cookie = session_cookie(&response)?;
    let _: AuthSessionResponse = read_json(response).await?;
    Ok(cookie)
}

async fn seed_open_access_fixture(
    pool: &PgPool,
    token: Option<&str>,
) -> TestResult<(i64, i64, i64)> {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name) VALUES ('coffee-legacy', 'Coffee Legacy') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format, schema_version) VALUES ($1, 'main', 'Main', 'yaml', 'v1') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name) VALUES ($1, 'prod', 'store-001', 'Store 001') RETURNING id",
    )
    .bind(project_id)
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
        ) VALUES ($1, $2, $3, '20260409.0001', 'log_level: info\n', 'abc123', 'yaml', 'initial', 'soft', 1)",
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .execute(pool)
    .await?;

    if let Some(token) = token {
        sqlx::query(
            "INSERT INTO deployment_credentials (deployment_instance_id, credential_name, token_hash) VALUES ($1, 'default', $2)",
        )
        .bind(deployment_id)
        .bind(server::auth::hash_bearer_token(token))
        .execute(pool)
        .await?;
    }

    Ok((project_id, config_file_id, deployment_id))
}

async fn resolve_main_config(
    app: &axum::Router,
    token: &str,
) -> TestResult<axum::response::Response> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())?,
        )
        .await?;
    Ok(response)
}

async fn reset_token(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
) -> TestResult<DeploymentTokenResetResponse> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/token/reset"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    read_json(response).await
}

#[tokio::test]
async fn reset_token_requires_session_cookie() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances/1/token/reset")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "auth_session_expired".to_owned(),
            message: "Authentication session expired".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn reset_token_returns_not_found_for_unknown_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances/999999/token/reset")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "deployment_instance_not_found".to_owned(),
            message: "deployment instance not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn reset_token_rotates_existing_default_credential_and_invalidates_old_token() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, Some(OLD_TOKEN)).await?;
    let cookie = login(&app).await?;

    let used_response = resolve_main_config(&app, OLD_TOKEN).await?;
    assert_eq!(used_response.status(), StatusCode::OK);
    let _: ResolveConfigResponse = read_json(used_response).await?;

    let payload = reset_token(&app, &cookie, deployment_id).await?;
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert_eq!(payload.credential_name, "default");
    assert_eq!(payload.token_preview, "mc_live_***");
    assert!(payload.token.starts_with("mc_live_"));

    let row = sqlx::query(
        "SELECT credential_name, token_hash, status, last_used_at IS NULL AS last_used_at_is_null FROM deployment_credentials WHERE deployment_instance_id = $1 AND credential_name = 'default'",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("credential_name"), "default");
    assert_eq!(
        row.get::<String, _>("token_hash"),
        server::auth::hash_bearer_token(&payload.token)
    );
    assert_eq!(row.get::<String, _>("status"), "active");
    assert!(row.get::<bool, _>("last_used_at_is_null"));

    let old_response = resolve_main_config(&app, OLD_TOKEN).await?;
    assert_eq!(old_response.status(), StatusCode::UNAUTHORIZED);
    let old_payload: ErrorResponse = read_json(old_response).await?;
    assert_eq!(
        old_payload,
        ErrorResponse {
            code: "invalid_token".to_owned(),
            message: "Invalid deployment token".to_owned(),
        }
    );

    let new_response = resolve_main_config(&app, &payload.token).await?;
    assert_eq!(new_response.status(), StatusCode::OK);
    let new_payload: ResolveConfigResponse = read_json(new_response).await?;
    assert_eq!(new_payload.release.revision, "20260409.0001");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn reset_token_creates_default_credential_when_missing() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, None).await?;
    let cookie = login(&app).await?;

    let payload = reset_token(&app, &cookie, deployment_id).await?;
    assert_eq!(payload.credential_name, "default");
    assert_eq!(payload.token_preview, "mc_live_***");
    assert!(payload.token.starts_with("mc_live_"));

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_credentials WHERE deployment_instance_id = $1",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);

    let response = resolve_main_config(&app, &payload.token).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let resolved: ResolveConfigResponse = read_json(response).await?;
    assert_eq!(resolved.config, "main");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn reset_token_only_keeps_latest_token_valid_after_multiple_rotations() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, Some(OLD_TOKEN)).await?;
    let cookie = login(&app).await?;

    let first = reset_token(&app, &cookie, deployment_id).await?;
    let second = reset_token(&app, &cookie, deployment_id).await?;

    let first_response = resolve_main_config(&app, &first.token).await?;
    assert_eq!(first_response.status(), StatusCode::UNAUTHORIZED);
    let first_payload: ErrorResponse = read_json(first_response).await?;
    assert_eq!(first_payload.code, "invalid_token");

    let second_response = resolve_main_config(&app, &second.token).await?;
    assert_eq!(second_response.status(), StatusCode::OK);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment_credentials WHERE deployment_instance_id = $1 AND credential_name = 'default'",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);

    teardown(&database_url, &schema, pool).await
}
