use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::{auth::AuthSessionResponse, draft::DraftResponse, release::ReleaseSummary};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping releases integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_releases_{nanos}")
}

fn with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options[search_path]={schema}")
}

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url() else {
        return Ok(None);
    };
    let schema = unique_schema_name();
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
    let body = to_bytes(response.into_body(), usize::MAX).await?;
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

async fn seed_project_config_deployment(pool: &PgPool) -> TestResult<(i64, i64, i64)> {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(pool)
    .await?;
    let config_file_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            format,
            schema_name,
            schema_version,
            sensitivity,
            status
        )
        VALUES ($1, 'main', 'Main Config', 'yaml', 'coffee-main', 'v1', 'normal', 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment,
            deployment_key,
            name,
            is_template,
            status
        )
        VALUES ($1, 'prod', 'store-001', 'Store 001', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    Ok((project_id, config_file_id, deployment_id))
}

async fn save_draft(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
    config_file_id: i64,
    content: &str,
    base_version: Option<i64>,
) -> TestResult<DraftResponse> {
    let body = if let Some(base_version) = base_version {
        format!(
            r#"{{"content":{},"format":"yaml","base_version":{base_version}}}"#,
            serde_json::to_string(content)?
        )
    } else {
        format!(
            r#"{{"content":{},"format":"yaml"}}"#,
            serde_json::to_string(content)?
        )
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))?,
        )
        .await?;

    read_json(response).await
}

#[tokio::test]
async fn publish_release_creates_release_from_current_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _draft = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        None,
    )
    .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id},"change_summary":"increase polling interval"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: ReleaseSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert_eq!(payload.config_file_id, config_file_id);
    assert_eq!(payload.format, "yaml");
    assert_eq!(payload.apply_mode, "soft");
    assert_eq!(
        payload.change_summary.as_deref(),
        Some("increase polling interval")
    );

    let row = sqlx::query(
        "SELECT revision, content, change_summary, published_by FROM releases WHERE id = $1",
    )
    .bind(payload.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("revision"), payload.revision);
    assert_eq!(row.get::<String, _>("content"), "poll_interval_ms: 5000\n");
    assert_eq!(
        row.get::<Option<String>, _>("change_summary").as_deref(),
        Some("increase polling interval")
    );
    assert!(row.get::<i64, _>("published_by") > 0);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_generates_new_revision_for_identical_draft_content() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _draft = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        None,
    )
    .await?;

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id}}}"#
                )))?,
        )
        .await?;
    let first: ReleaseSummary = read_json(first).await?;

    let second = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id}}}"#
                )))?,
        )
        .await?;
    let second: ReleaseSummary = read_json(second).await?;

    assert_ne!(first.revision, second.revision);
    assert_eq!(first.content_hash, second.content_hash);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_returns_draft_not_found_when_current_draft_is_missing() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id}}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "draft_not_found".to_owned(),
            message: "draft not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}
