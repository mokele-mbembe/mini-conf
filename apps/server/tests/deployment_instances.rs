use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    auth::AuthSessionResponse,
    deployment_instance::{
        DeploymentBundlePreviewResponse, DeploymentInstanceListResponse, DeploymentInstanceSummary,
    },
};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn install_admin_project_membership_trigger(pool: &PgPool) -> TestResult {
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION auto_grant_test_admin_project_member()
        RETURNS TRIGGER AS $$
        BEGIN
            INSERT INTO project_members (project_id, user_id, role)
            SELECT NEW.id, id, 'admin'
            FROM users
            WHERE username = 'admin'
              AND status = 'active'
            ON CONFLICT (project_id, user_id) DO NOTHING;
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        DROP TRIGGER IF EXISTS trg_auto_grant_test_admin_project_member ON projects;
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER trg_auto_grant_test_admin_project_member
        AFTER INSERT ON projects
        FOR EACH ROW
        EXECUTE FUNCTION auto_grant_test_admin_project_member();
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("deployment instances") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_deployments");
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
    install_admin_project_membership_trigger(&pool).await?;

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
    let csrf_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/csrf")
                .body(Body::empty())?,
        )
        .await?;
    let csrf_cookie = session_cookie(&csrf_response)?;
    let csrf_token = csrf_cookie
        .strip_prefix("mini_conf_csrf=")
        .ok_or("set-cookie should contain a csrf cookie")?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::COOKIE, &csrf_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf_token)
                .body(Body::from(
                    r#"{"username":"admin","password":"admin123456"}"#,
                ))?,
        )
        .await?;

    let cookie = session_cookie(&response)?;
    let _: AuthSessionResponse = read_json(response).await?;
    Ok(cookie)
}

async fn seed_project(pool: &PgPool, code: &str, name: &str) -> TestResult<i64> {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ($1, $2, 'active') RETURNING id",
    )
    .bind(code)
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(project_id)
}

async fn seed_config_file(pool: &PgPool, project_id: i64, code: &str) -> TestResult<i64> {
    let config_file_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            format,
            sensitivity,
            status
        )
        VALUES ($1, $2, $3, 'yaml', 'normal', 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(code)
    .bind(format!("{code} config"))
    .fetch_one(pool)
    .await?;
    Ok(config_file_id)
}

async fn seed_project_environment(
    pool: &PgPool,
    project_id: i64,
    code: &str,
    name: &str,
    status: &str,
    sort_order: i32,
) -> TestResult<i64> {
    let environment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO project_environments (
            project_id,
            code,
            name,
            status,
            sort_order
        )
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (project_id, code)
        DO UPDATE SET
            name = EXCLUDED.name,
            status = EXCLUDED.status,
            sort_order = EXCLUDED.sort_order,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(code)
    .bind(name)
    .bind(status)
    .bind(sort_order)
    .fetch_one(pool)
    .await?;
    Ok(environment_id)
}

async fn seed_template_deployment(
    pool: &PgPool,
    project_id: i64,
    deployment_key: &str,
) -> TestResult<i64> {
    let environment_id =
        seed_project_environment(pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment_id,
            deployment_key,
            name,
            is_template,
            status
        )
        VALUES ($1, $2, $3, 'Template Store', true, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .bind(deployment_key)
    .fetch_one(pool)
    .await?;
    Ok(deployment_id)
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
    let project_a_prod_env =
        seed_project_environment(&pool, project_a, "prod", "Production", "active", 10).await?;
    let project_a_staging_env =
        seed_project_environment(&pool, project_a, "staging", "Staging", "active", 20).await?;
    let project_b_prod_env =
        seed_project_environment(&pool, project_b, "prod", "Production", "active", 10).await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES
            ($1, $2, 'store-001', 'Store 001', false, 'active'),
            ($1, $3, 'store-stg', 'Store Staging', false, 'active'),
            ($4, $5, 'store-ops', 'Store Ops', true, 'inactive')
        "#,
    )
    .bind(project_a)
    .bind(project_a_prod_env)
    .bind(project_a_staging_env)
    .bind(project_b)
    .bind(project_b_prod_env)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_a}&environment_id={project_a_prod_env}&status=active"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.total, 1);
    assert_eq!(payload.page, 1);
    assert_eq!(payload.page_size, 20);
    assert_eq!(payload.items[0].project_id, project_a);
    assert_eq!(payload.items[0].environment_id, project_a_prod_env);
    assert_eq!(payload.items[0].environment_code, "prod");
    assert_eq!(payload.items[0].environment_name, "Production");
    assert_eq!(payload.items[0].deployment_key, "store-001");

    let keyword_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_a}&keyword=001"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(keyword_response.status(), StatusCode::OK);
    let keyword_payload: DeploymentInstanceListResponse = read_json(keyword_response).await?;
    assert_eq!(keyword_payload.items.len(), 1);
    assert_eq!(keyword_payload.total, 1);
    assert_eq!(keyword_payload.items[0].deployment_key, "store-001");

    let paged_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_a}&status=active&page=2&page_size=1"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(paged_response.status(), StatusCode::OK);
    let paged_payload: DeploymentInstanceListResponse = read_json(paged_response).await?;
    assert_eq!(paged_payload.total, 2);
    assert_eq!(paged_payload.page, 2);
    assert_eq!(paged_payload.page_size, 1);
    assert_eq!(paged_payload.items.len(), 1);
    assert_eq!(paged_payload.items[0].deployment_key, "store-stg");

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
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;

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
                    r#"{{"project_id":{project_id},"environment_id":{environment_id},"deployment_key":"store-001","name":"Store 001","description":"hangzhou store 001","is_template":false}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.environment_id, environment_id);
    assert_eq!(payload.environment_code, "prod");
    assert_eq!(payload.environment_name, "Production");
    assert_eq!(payload.deployment_key, "store-001");
    assert!(!payload.is_template);
    assert_eq!(payload.status, "inactive");

    let row = sqlx::query(
        "SELECT environment_id, deployment_key, name, is_template, status FROM deployment_instances WHERE id = $1",
    )
    .bind(payload.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<i64, _>("environment_id"), environment_id);
    assert_eq!(row.get::<String, _>("deployment_key"), "store-001");
    assert_eq!(row.get::<String, _>("name"), "Store 001");
    assert!(!row.get::<bool, _>("is_template"));
    assert_eq!(row.get::<String, _>("status"), "inactive");

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
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    sqlx::query(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status) VALUES ($1, $2, 'store-001', 'Store 001', false, 'active')",
    )
    .bind(project_id)
    .bind(environment_id)
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
                    r#"{{"project_id":{project_id},"environment_id":{environment_id},"deployment_key":"store-001","name":"Duplicate Store","is_template":false}}"#
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
                    r#"{"project_id":999999,"environment_id":999999,"deployment_key":"store-001","name":"Store 001","is_template":false}"#,
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
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status) VALUES ($1, $2, 'store-001', 'Store 001', false, 'active') RETURNING id",
    )
    .bind(project_id)
    .bind(environment_id)
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
    assert_eq!(payload.environment_id, environment_id);
    assert_eq!(payload.environment_code, "prod");
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
    let prod_environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let staging_environment_id =
        seed_project_environment(&pool, project_id, "staging", "Staging", "active", 20).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status) VALUES ($1, $2, 'store-001', 'Store 001', false, 'active') RETURNING id",
    )
    .bind(project_id)
    .bind(prod_environment_id)
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
                .body(Body::from(
                    format!(
                        r#"{{"environment_id":{staging_environment_id},"deployment_key":"store-stg","name":"Store Staging","description":"updated staging deployment"}}"#
                    ),
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.environment_id, staging_environment_id);
    assert_eq!(payload.environment_code, "staging");
    assert_eq!(payload.environment_name, "Staging");
    assert_eq!(payload.deployment_key, "store-stg");
    assert!(!payload.is_template);
    assert_eq!(payload.status, "active");

    let row = sqlx::query(
        "SELECT environment_id, deployment_key, name, is_template, status FROM deployment_instances WHERE id = $1",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<i64, _>("environment_id"), staging_environment_id);
    assert_eq!(row.get::<String, _>("deployment_key"), "store-stg");
    assert_eq!(row.get::<String, _>("name"), "Store Staging");
    assert!(!row.get::<bool, _>("is_template"));
    assert_eq!(row.get::<String, _>("status"), "active");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn update_deployment_instance_rejects_immutable_fields() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status) VALUES ($1, $2, 'store-001', 'Store 001', false, 'active') RETURNING id",
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    for body in [
        format!(
            r#"{{"project_id":{project_id},"environment_id":{environment_id},"deployment_key":"store-001","name":"Store 001"}}"#
        ),
        format!(
            r#"{{"environment_id":{environment_id},"deployment_key":"store-001","name":"Store 001","is_template":true}}"#
        ),
        format!(
            r#"{{"environment_id":{environment_id},"deployment_key":"store-001","name":"Store 001","status":"inactive"}}"#
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/deployment-instances/{deployment_id}"))
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let payload: ErrorResponse = read_json(response).await?;
        assert_eq!(payload.code, "invalid_request");
    }

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
    let prod_environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let staging_environment_id =
        seed_project_environment(&pool, project_id, "staging", "Staging", "active", 20).await?;
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
                    r#"{{"project_id":{project_id},"environment_id":{prod_environment_id},"deployment_key":"store-001","name":"Store 001","description":"hangzhou store 001","is_template":false}}"#
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
                    "/api/deployment-instances?project_id={project_id}&environment_id={prod_environment_id}&status=inactive"
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
                    r#"{{"environment_id":{staging_environment_id},"deployment_key":"store-stg","name":"Store Staging","description":"updated staging deployment"}}"#
                )))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: DeploymentInstanceSummary = read_json(update_response).await?;
    assert_eq!(updated.environment_id, staging_environment_id);
    assert_eq!(updated.environment_code, "staging");
    assert_eq!(updated.deployment_key, "store-stg");
    assert_eq!(updated.status, "inactive");

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
    assert_eq!(detail_after_update.environment_id, staging_environment_id);
    assert_eq!(detail_after_update.environment_code, "staging");
    assert_eq!(detail_after_update.deployment_key, "store-stg");
    assert_eq!(detail_after_update.status, "inactive");

    let row = sqlx::query(
        "SELECT environment_id, deployment_key, status FROM deployment_instances WHERE id = $1",
    )
    .bind(created.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<i64, _>("environment_id"), staging_environment_id);
    assert_eq!(row.get::<String, _>("deployment_key"), "store-stg");
    assert_eq!(row.get::<String, _>("status"), "inactive");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn clone_deployment_instance_copies_template_drafts() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let config_file_id = seed_config_file(&pool, project_id, "main").await?;
    let template_id = seed_template_deployment(&pool, project_id, "store-template").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let editor_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO drafts (
            project_id,
            config_file_id,
            deployment_instance_id,
            content,
            content_hash,
            format,
            version,
            editor_user_id
        )
        VALUES ($1, $2, $3, $4, repeat('a', 64), 'yaml', 4, $5)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(template_id)
    .bind("poll_interval_ms: 5000\n")
    .bind(editor_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{template_id}/clone"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"deployment_key":"store-002","name":"Store 002","environment_id":{environment_id},"description":"cloned from template","clone_source":"draft"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.template_source_id, Some(template_id));
    assert!(!payload.is_template);

    let row =
        sqlx::query("SELECT content, format, version FROM drafts WHERE deployment_instance_id = $1 AND config_file_id = $2")
    .bind(payload.id)
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("content"), "poll_interval_ms: 5000\n");
    assert_eq!(row.get::<String, _>("format"), "yaml");
    assert_eq!(row.get::<i64, _>("version"), 1);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn clone_deployment_instance_rejects_latest_release_source() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let _config_file_id = seed_config_file(&pool, project_id, "main").await?;
    let template_id = seed_template_deployment(&pool, project_id, "store-template").await?;
    let staging_environment_id =
        seed_project_environment(&pool, project_id, "staging", "Staging", "active", 20).await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{template_id}/clone"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"deployment_key":"store-003","name":"Store 003","environment_id":{staging_environment_id},"clone_source":"latest_release"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "invalid_request".to_owned(),
            message: "invalid deployment clone source".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn preview_bundle_prefers_draft_and_marks_missing_required() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let main_config_id = seed_config_file(&pool, project_id, "main").await?;
    let vision_config_id = seed_config_file(&pool, project_id, "vision").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    sqlx::query("UPDATE config_files SET is_required = TRUE WHERE id = $1")
        .bind(vision_config_id)
        .execute(&pool)
        .await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment_id,
            deployment_key,
            name,
            is_template,
            status
        )
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;
    let editor_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO drafts (
            project_id,
            config_file_id,
            deployment_instance_id,
            content,
            content_hash,
            format,
            version,
            editor_user_id
        )
        VALUES ($1, $2, $3, $4, repeat('a', 64), 'yaml', 2, $5)
        "#,
    )
    .bind(project_id)
    .bind(main_config_id)
    .bind(deployment_id)
    .bind("poll_interval_ms: 5000\n")
    .bind(editor_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/preview-bundle"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentBundlePreviewResponse = read_json(response).await?;
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert_eq!(payload.open_bundle_preview.configs.len(), 1);
    assert_eq!(payload.open_bundle_preview.configs[0].config, "main");
    assert_eq!(payload.open_bundle_preview.configs[0].revision, "draft-v2");
    assert_eq!(payload.items.len(), 2);
    assert!(
        payload
            .items
            .iter()
            .any(|item| item.code == "main" && item.source == "draft" && item.status == "ready")
    );
    assert!(payload.items.iter().any(|item| item.code == "vision"
        && item.source == "none"
        && item.status == "missing_required"));

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_deployment_instances_filters_by_is_template() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let prod_env =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let staging_env =
        seed_project_environment(&pool, project_id, "staging", "Staging", "active", 20).await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES
            ($1, $2, 'tmpl-a', 'Template A', true, 'inactive'),
            ($1, $2, 'tmpl-b', 'Template B', true, 'inactive'),
            ($1, $3, 'store-001', 'Store 001', false, 'active'),
            ($1, $3, 'store-002', 'Store 002', false, 'inactive')
        "#,
    )
    .bind(project_id)
    .bind(prod_env)
    .bind(staging_env)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;

    // is_template=true: only templates
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&is_template=true"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 2);
    assert_eq!(payload.items.len(), 2);
    assert!(payload.items.iter().all(|item| item.is_template));

    // is_template=false: only non-templates
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&is_template=false"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 2);
    assert_eq!(payload.items.len(), 2);
    assert!(payload.items.iter().all(|item| !item.is_template));

    // no is_template: returns all 4
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/deployment-instances?project_id={project_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 4);
    assert_eq!(payload.items.len(), 4);

    // is_template combined with keyword
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&is_template=true&keyword=tmpl-a"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 1);
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].deployment_key, "tmpl-a");
    assert!(payload.items[0].is_template);

    // is_template combined with environment_id
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&is_template=false&environment_id={staging_env}"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 2);
    assert_eq!(payload.items.len(), 2);
    assert!(
        payload
            .items
            .iter()
            .all(|item| !item.is_template && item.environment_id == staging_env)
    );

    // is_template combined with status
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&is_template=false&status=active"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentInstanceListResponse = read_json(response).await?;
    assert_eq!(payload.total, 1);
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].deployment_key, "store-001");
    assert!(!payload.items[0].is_template);
    assert_eq!(payload.items[0].status, "active");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn archive_restore_delete_flow_supports_key_reuse() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'inactive')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;

    let archive_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{deployment_id}/archive"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(archive_response.status(), StatusCode::OK);
    let archived: DeploymentInstanceSummary = read_json(archive_response).await?;
    assert!(archived.is_archived);
    assert_eq!(archived.status, "inactive");

    let restore_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{deployment_id}/restore"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(restore_response.status(), StatusCode::OK);
    let restored: DeploymentInstanceSummary = read_json(restore_response).await?;
    assert!(!restored.is_archived);
    assert_eq!(restored.status, "inactive");

    let archive_again = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{deployment_id}/archive"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(archive_again.status(), StatusCode::OK);

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let deleted_at: Option<String> = sqlx::query_scalar(
        r#"
        SELECT to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        FROM deployment_instances
        WHERE id = $1
        "#,
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert!(deleted_at.is_some());

    let recreate_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/deployment-instances")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"environment_id":{environment_id},"deployment_key":"store-001","name":"Store 001 New","is_template":false}}"#
                )))?,
        )
        .await?;
    assert_eq!(recreate_response.status(), StatusCode::CREATED);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn archive_requires_inactive_status() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{deployment_id}/archive"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "deployment_instance_archive_conflict");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn delete_requires_archived_and_inactive() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'inactive')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "deployment_instance_delete_conflict");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_deployment_instances_visibility_filter_excludes_deleted() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            deployment_uid,
            project_id,
            environment_id,
            deployment_key,
            name,
            is_template,
            status,
            is_archived,
            archived_at,
            deleted_at
        )
        VALUES
            (gen_random_uuid(), $1, $2, 'current-1', 'Current 1', false, 'inactive', false, NULL, NULL),
            (gen_random_uuid(), $1, $2, 'archived-1', 'Archived 1', false, 'inactive', true, NOW(), NULL),
            (gen_random_uuid(), $1, $2, 'deleted-1', 'Deleted 1', false, 'inactive', true, NOW(), NOW())
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;

    let current_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/deployment-instances?project_id={project_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(current_response.status(), StatusCode::OK);
    let current_payload: DeploymentInstanceListResponse = read_json(current_response).await?;
    assert_eq!(current_payload.total, 1);
    assert_eq!(current_payload.items[0].deployment_key, "current-1");

    let archived_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&visibility_filter=archived"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(archived_response.status(), StatusCode::OK);
    let archived_payload: DeploymentInstanceListResponse = read_json(archived_response).await?;
    assert_eq!(archived_payload.total, 1);
    assert_eq!(archived_payload.items[0].deployment_key, "archived-1");

    let all_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&visibility_filter=all"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(all_response.status(), StatusCode::OK);
    let all_payload: DeploymentInstanceListResponse = read_json(all_response).await?;
    assert_eq!(all_payload.total, 2);
    assert!(
        all_payload
            .items
            .iter()
            .all(|item| item.deployment_key != "deleted-1")
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn archived_deployment_cannot_activate() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (deployment_uid, project_id, environment_id, deployment_key, name, is_template, status, is_archived, archived_at)
        VALUES (gen_random_uuid(), $1, $2, 'store-001', 'Store 001', false, 'inactive', true, NOW())
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/activate"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "deployment_instance_archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_deployment_instance_returns_404_for_deleted_tombstone() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status,
            is_archived, archived_at, deleted_at)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'inactive', true, NOW(), NOW())
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "deployment_instance_not_found");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn delete_tombstone_cleans_up_drafts_credentials_and_saved_versions() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let config_file_id = seed_config_file(&pool, project_id, "main").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'inactive')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(&pool)
    .await?;

    // Seed a draft.
    let admin_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO drafts (project_id, config_file_id, deployment_instance_id, content, content_hash, format, version, editor_user_id)
        VALUES ($1, $2, $3, 'key: val\n', repeat('a', 64), 'yaml', 1, $4)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(admin_user_id)
    .execute(&pool)
    .await?;

    // Seed a saved version.
    sqlx::query(
        r#"
        INSERT INTO draft_saved_versions (
            project_id,
            config_file_id,
            deployment_instance_id,
            title,
            content,
            content_hash,
            format,
            source_draft_version,
            created_by
        )
        VALUES ($1, $2, $3, 'Cleanup checkpoint', 'key: val\n', repeat('a', 64), 'yaml', 1, $4)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(admin_user_id)
    .execute(&pool)
    .await?;

    // Seed a deployment credential.
    sqlx::query(
        r#"
        INSERT INTO deployment_credentials (deployment_instance_id, token_hash, status)
        VALUES ($1, repeat('b', 64), 'active')
        "#,
    )
    .bind(deployment_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;

    // Archive first (required before delete).
    let archive_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{deployment_id}/archive"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(archive_resp.status(), StatusCode::OK);

    // Delete tombstone.
    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/deployment-instances/{deployment_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

    // Draft should be gone.
    let draft_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM drafts WHERE deployment_instance_id = $1")
            .bind(deployment_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(draft_count, 0, "drafts should be deleted after tombstone");

    // Saved version should be soft-deleted.
    let live_sv_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM draft_saved_versions WHERE deployment_instance_id = $1 AND deleted_at IS NULL",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        live_sv_count, 0,
        "saved versions should be soft-deleted after tombstone"
    );

    // Credential should be inactive.
    let active_cred_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM deployment_credentials WHERE deployment_instance_id = $1 AND status = 'active'",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        active_cred_count, 0,
        "credentials should be inactivated after tombstone"
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_deployment_instances_rejects_invalid_visibility_filter() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active", 10).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/deployment-instances?project_id={project_id}&environment_id={environment_id}&visibility_filter=bogus"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "invalid_request");

    teardown(&database_url, &schema, pool).await
}
