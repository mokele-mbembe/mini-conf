use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use schema::{
    auth::AuthSessionResponse,
    deployment_instance::{DeploymentInstanceListResponse, DeploymentInstanceSummary},
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
            eprintln!("skipping deployment instances integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_deployments_{nanos}")
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
async fn list_deployment_instances_filters_by_project_and_environment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_a: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let project_b: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('store-os', 'Store OS', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_instances (project_id, environment, deployment_key, name, is_template, status)
        VALUES
            ($1, 'prod', 'store-001', 'Store 001', false, 'active'),
            ($1, 'staging', 'store-stg', 'Store Staging', false, 'active'),
            ($2, 'prod', 'store-ops', 'Store Ops', true, 'archived')
        "#,
    )
    .bind(project_a)
    .bind(project_b)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_a}&environment=prod&status=active"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].project_id, project_a);
    assert_eq!(payload.items[0].environment, "prod");
    assert_eq!(payload.items[0].deployment_key, "store-001");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_deployment_instance_creates_row() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment":"prod","deployment_key":"store-001","name":"Store 001","description":"hangzhou store 001","is_template":false}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.environment, "prod");
    assert_eq!(payload.deployment_key, "store-001");
    assert!(!payload.is_template);

    let row = sqlx::query(
        "SELECT environment, deployment_key, name, is_template, status FROM deployment_instances WHERE id = $1",
    )
    .bind(payload.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("environment"), "prod");
    assert_eq!(row.get::<String, _>("deployment_key"), "store-001");
    assert_eq!(row.get::<String, _>("name"), "Store 001");
    assert!(!row.get::<bool, _>("is_template"));
    assert_eq!(row.get::<String, _>("status"), "active");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_deployment_instance_rejects_duplicate_key_in_same_project_environment() -> TestResult
{
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name, is_template, status) VALUES ($1, 'prod', 'store-001', 'Store 001', false, 'active')",
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment":"prod","deployment_key":"store-001","name":"Duplicate Store","is_template":false}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "deployment_key_conflict".to_owned(),
            message: "deployment key already exists in project environment".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_deployment_instance_returns_project_not_found_for_unknown_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"project_id":999999,"environment":"prod","deployment_key":"store-001","name":"Store 001","is_template":false}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "project_not_found".to_owned(),
            message: "project not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_deployment_instance_returns_detail_for_authenticated_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name, is_template, status) VALUES ($1, 'prod', 'store-001', 'Store 001', false, 'active') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.id, deployment_id);
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.deployment_key, "store-001");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn update_deployment_instance_updates_existing_row() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment, deployment_key, name, is_template, status) VALUES ($1, 'prod', 'store-001', 'Store 001', false, 'active') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment":"staging","deployment_key":"store-stg","name":"Store Staging","description":"updated staging deployment","is_template":true,"status":"archived"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.environment, "staging");
    assert_eq!(payload.deployment_key, "store-stg");
    assert!(payload.is_template);
    assert_eq!(payload.status, "archived");

    let row = sqlx::query(
        "SELECT environment, deployment_key, name, is_template, status FROM deployment_instances WHERE id = $1",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("environment"), "staging");
    assert_eq!(row.get::<String, _>("deployment_key"), "store-stg");
    assert_eq!(row.get::<String, _>("name"), "Store Staging");
    assert!(row.get::<bool, _>("is_template"));
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn deployment_instance_crud_flow_persists_changes_across_endpoints() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let cookie = login(&app).await?;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment":"prod","deployment_key":"store-001","name":"Store 001","description":"hangzhou store 001","is_template":false}}"#
                )))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: DeploymentInstanceSummary = read_json(create_response).await?;

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&environment=prod&status=active"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: DeploymentInstanceListResponse = read_json(list_response).await?;
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id, created.id);

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/deployment-instances/{}", created.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: DeploymentInstanceSummary = read_json(detail_response).await?;
    assert_eq!(detail.deployment_key, "store-001");

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/deployment-instances/{}", created.id))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment":"staging","deployment_key":"store-stg","name":"Store Staging","description":"updated staging deployment","is_template":true,"status":"archived"}}"#
                )))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: DeploymentInstanceSummary = read_json(update_response).await?;
    assert_eq!(updated.environment, "staging");
    assert_eq!(updated.deployment_key, "store-stg");
    assert_eq!(updated.status, "archived");

    let detail_after_update = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/deployment-instances/{}", created.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_after_update.status(), StatusCode::OK);
    let detail_after_update: DeploymentInstanceSummary = read_json(detail_after_update).await?;
    assert_eq!(detail_after_update.environment, "staging");
    assert_eq!(detail_after_update.deployment_key, "store-stg");
    assert_eq!(detail_after_update.status, "archived");

    let row = sqlx::query(
        "SELECT environment, deployment_key, status FROM deployment_instances WHERE id = $1",
    )
    .bind(created.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("environment"), "staging");
    assert_eq!(row.get::<String, _>("deployment_key"), "store-stg");
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}
