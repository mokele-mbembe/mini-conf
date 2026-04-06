use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::{
    auth::AuthSessionResponse,
    project::{ProjectListResponse, ProjectSummary},
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
            eprintln!("skipping projects integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_projects_{nanos}")
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

#[tokio::test]
async fn list_projects_requires_session_cookie() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects")
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
async fn list_projects_returns_active_projects_for_authenticated_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    sqlx::query(
        r#"
        INSERT INTO projects (code, name, description, status)
        VALUES
            ('coffee-legacy', 'Coffee Legacy', 'Retail edge rollout', 'active'),
            ('coffee-shadow', 'Coffee Shadow', NULL, 'archived'),
            ('store-os', 'Store OS', 'Ops control plane', 'active')
        "#,
    )
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ProjectListResponse = read_json(response).await?;

    assert_eq!(payload.items.len(), 2);
    assert_eq!(payload.items[0].code, "coffee-legacy");
    assert_eq!(payload.items[0].name, "Coffee Legacy");
    assert_eq!(
        payload.items[0].description.as_deref(),
        Some("Retail edge rollout")
    );
    assert_eq!(payload.items[1].code, "store-os");
    assert_eq!(payload.items[1].status, "active");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_project_creates_active_project_for_authenticated_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"store-os","name":"Store OS","description":"Ops control plane"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: ProjectSummary = read_json(response).await?;
    assert_eq!(payload.code, "store-os");
    assert_eq!(payload.status, "active");

    let row = sqlx::query("SELECT code, name, description, status FROM projects WHERE code = $1")
        .bind("store-os")
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.get::<String, _>("code"), "store-os");
    assert_eq!(row.get::<String, _>("name"), "Store OS");
    assert_eq!(
        row.get::<Option<String>, _>("description").as_deref(),
        Some("Ops control plane")
    );
    assert_eq!(row.get::<String, _>("status"), "active");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_project_rejects_duplicate_project_code() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    sqlx::query("INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active')")
        .execute(&pool)
        .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"coffee-legacy","name":"Coffee Legacy Duplicate"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "project_code_conflict".to_owned(),
            message: "project code already exists".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}
