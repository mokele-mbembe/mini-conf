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
            schema_name,
            schema_version,
            sensitivity,
            status
        )
        VALUES ($1, $2, $3, 'yaml', 'coffee-main', 'v1', 'normal', 'active')
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

async fn seed_template_deployment(
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
        VALUES ($1, 'prod', $2, 'Template Store', true, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
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

#[tokio::test]
async fn clone_deployment_instance_copies_template_drafts() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let config_file_id = seed_config_file(&pool, project_id, "main").await?;
    let template_id = seed_template_deployment(&pool, project_id, "store-template").await?;
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
            schema_version,
            version,
            editor_user_id
        )
        VALUES ($1, $2, $3, $4, repeat('a', 64), 'yaml', 'v1', 4, $5)
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
                .body(Body::from(
                    r#"{"deployment_key":"store-002","name":"Store 002","environment":"prod","description":"cloned from template","clone_source":"draft"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: DeploymentInstanceSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.template_source_id, Some(template_id));
    assert!(!payload.is_template);

    let row = sqlx::query(
        "SELECT content, format, schema_version, version FROM drafts WHERE deployment_instance_id = $1 AND config_file_id = $2",
    )
    .bind(payload.id)
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("content"), "poll_interval_ms: 5000\n");
    assert_eq!(row.get::<String, _>("format"), "yaml");
    assert_eq!(
        row.get::<Option<String>, _>("schema_version").as_deref(),
        Some("v1")
    );
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

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{template_id}/clone"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"deployment_key":"store-003","name":"Store 003","environment":"staging","clone_source":"latest_release"}"#,
                ))?,
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
    sqlx::query("UPDATE config_files SET is_required = TRUE WHERE id = $1")
        .bind(vision_config_id)
        .execute(&pool)
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
            schema_version,
            version,
            editor_user_id
        )
        VALUES ($1, $2, $3, $4, repeat('a', 64), 'yaml', 'v1', 2, $5)
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
