use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::{
    auth::AuthSessionResponse,
    draft::{DraftCloneResponse, DraftResponse},
};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping drafts integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_drafts_{nanos}")
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

async fn seed_second_deployment(
    pool: &PgPool,
    project_id: i64,
    deployment_key: &str,
) -> TestResult<i64> {
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
        VALUES ($1, 'prod', $2, $3, false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(deployment_key)
    .bind(format!("{deployment_key} deployment"))
    .fetch_one(pool)
    .await?;
    Ok(deployment_id)
}

#[tokio::test]
async fn get_draft_returns_not_found_when_missing() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
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

#[tokio::test]
async fn put_draft_creates_and_get_returns_current_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"poll_interval_ms: 5000\n","format":"yaml"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DraftResponse = read_json(response).await?;
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert_eq!(payload.config_file_id, config_file_id);
    assert_eq!(payload.version, 1);
    assert_eq!(payload.schema_version.as_deref(), Some("v1"));

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DraftResponse = read_json(response).await?;
    assert_eq!(payload.content, "poll_interval_ms: 5000\n");
    assert_eq!(payload.version, 1);

    let row = sqlx::query(
        "SELECT content, format, version FROM drafts WHERE deployment_instance_id = $1 AND config_file_id = $2",
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("content"), "poll_interval_ms: 5000\n");
    assert_eq!(row.get::<String, _>("format"), "yaml");
    assert_eq!(row.get::<i64, _>("version"), 1);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn put_draft_updates_existing_draft_and_increments_version() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"poll_interval_ms: 5000\n","format":"yaml"}"#,
                ))?,
        )
        .await?;
    let created: DraftResponse = read_json(response).await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"content":"poll_interval_ms: 8000\n","format":"yaml","base_version":{}}}"#,
                    created.version
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DraftResponse = read_json(response).await?;
    assert_eq!(payload.content, "poll_interval_ms: 8000\n");
    assert_eq!(payload.version, 2);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn put_draft_rejects_stale_base_version() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"poll_interval_ms: 5000\n","format":"yaml","base_version":9}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "draft_version_conflict".to_owned(),
            message: "draft version conflict".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn put_draft_rejects_format_mismatch() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"{\"poll_interval_ms\":5000}","format":"json"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "draft_validation_failed".to_owned(),
            message: "draft format must match config file format".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn clone_draft_copies_latest_release_into_target_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, source_deployment_id) =
        seed_project_config_deployment(&pool).await?;
    let target_deployment_id = seed_second_deployment(&pool, project_id, "store-002").await?;
    let publisher_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO releases (
            project_id,
            config_file_id,
            deployment_instance_id,
            revision,
            content,
            content_hash,
            format,
            change_summary,
            diff_summary,
            apply_mode,
            published_by
        )
        VALUES
            ($1, $2, $3, '20260407.0001', $4, repeat('c', 64), 'yaml', NULL, NULL, 'soft', $5)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(source_deployment_id)
    .bind("poll_interval_ms: 7000\n")
    .bind(publisher_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/drafts/{target_deployment_id}/{config_file_id}/clone"
                ))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"source_deployment_instance_id":{source_deployment_id},"source_kind":"latest_release"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DraftCloneResponse = read_json(response).await?;
    assert_eq!(payload.source_kind, "latest_release");
    assert_eq!(payload.source_deployment_instance_id, source_deployment_id);
    assert_eq!(payload.draft.deployment_instance_id, target_deployment_id);
    assert_eq!(payload.draft.content, "poll_interval_ms: 7000\n");
    assert_eq!(payload.draft.version, 1);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn clone_draft_overwrites_existing_target_and_increments_version() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, source_deployment_id) =
        seed_project_config_deployment(&pool).await?;
    let target_deployment_id = seed_second_deployment(&pool, project_id, "store-002").await?;
    let cookie = login(&app).await?;

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/drafts/{source_deployment_id}/{config_file_id}"
                ))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"poll_interval_ms: 5000\n","format":"yaml"}"#,
                ))?,
        )
        .await?;
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/drafts/{target_deployment_id}/{config_file_id}"
                ))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"poll_interval_ms: 3000\n","format":"yaml"}"#,
                ))?,
        )
        .await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/drafts/{target_deployment_id}/{config_file_id}/clone"
                ))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"source_deployment_instance_id":{source_deployment_id},"source_kind":"draft"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DraftCloneResponse = read_json(response).await?;
    assert_eq!(payload.draft.content, "poll_interval_ms: 5000\n");
    assert_eq!(payload.draft.version, 2);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn clone_draft_rejects_cross_project_source() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, config_file_id, target_deployment_id) =
        seed_project_config_deployment(&pool).await?;
    let other_project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('store-os', 'Store OS', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let source_deployment_id =
        seed_second_deployment(&pool, other_project_id, "store-foreign").await?;
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/drafts/{target_deployment_id}/{config_file_id}/clone"
                ))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"source_deployment_instance_id":{source_deployment_id},"source_kind":"draft"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "draft_clone_cross_project_forbidden".to_owned(),
            message: "draft clone source must be in the same project".to_owned(),
        }
    );
    teardown(&database_url, &schema, pool).await
}
