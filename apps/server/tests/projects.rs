use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    admin_project::CreatePlatformProjectResponse,
    auth::AuthSessionResponse,
    project::{ProjectListResponse, ProjectSummary},
};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
use sqlx::{Connection, Executor, PgConnection, PgPool, Row};
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn setup_app() -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url("projects") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_projects");
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

async fn admin_user_id(pool: &PgPool) -> TestResult<i64> {
    let user_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
        .fetch_one(pool)
        .await?;
    Ok(user_id)
}

async fn grant_admin_membership(pool: &PgPool, project_id: i64) -> TestResult {
    let user_id = admin_user_id(pool).await?;
    sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, 'admin')
        ON CONFLICT (project_id, user_id) DO NOTHING
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn list_projects_requires_session_cookie() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let response = app
        .clone()
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
async fn list_projects_returns_visible_projects_and_supports_status_filter() -> TestResult {
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
    for code in ["coffee-legacy", "coffee-shadow", "store-os"] {
        let project_id: i64 = sqlx::query_scalar("SELECT id FROM projects WHERE code = $1")
            .bind(code)
            .fetch_one(&pool)
            .await?;
        grant_admin_membership(&pool, project_id).await?;
    }

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ProjectListResponse = read_json(response).await?;

    assert_eq!(payload.items.len(), 3);
    assert_eq!(payload.items[0].code, "coffee-legacy");
    assert_eq!(payload.items[1].code, "coffee-shadow");
    assert_eq!(payload.items[1].status, "archived");
    assert_eq!(payload.items[2].code, "store-os");

    let archived_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects?status=archived")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(archived_response.status(), StatusCode::OK);
    let archived_payload: ProjectListResponse = read_json(archived_response).await?;
    assert_eq!(archived_payload.items.len(), 1);
    assert_eq!(payload.items[0].code, "coffee-legacy");
    assert_eq!(payload.items[0].name, "Coffee Legacy");
    assert_eq!(
        payload.items[0].description.as_deref(),
        Some("Retail edge rollout")
    );
    assert_eq!(archived_payload.items[0].code, "coffee-shadow");
    assert_eq!(archived_payload.items[0].status, "archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_project_alias_creates_active_project_for_explicit_initial_admin() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let admin_user_id = admin_user_id(&pool).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"store-os","name":"Store OS","description":"Ops control plane","initial_admin_user_id":{admin_user_id}}}"#,
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: CreatePlatformProjectResponse = read_json(response).await?;
    assert_eq!(payload.project.code, "store-os");
    assert_eq!(payload.project.status, "active");

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
    let admin_user_id = admin_user_id(&pool).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"coffee-legacy","name":"Coffee Legacy Duplicate","initial_admin_user_id":{admin_user_id}}}"#,
                )))?,
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

#[tokio::test]
async fn get_project_returns_project_detail_for_authenticated_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let row = sqlx::query(
        "INSERT INTO projects (code, name, description, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'Retail edge rollout', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let project_id: i64 = row.get("id");
    grant_admin_membership(&pool, project_id).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ProjectSummary = read_json(response).await?;
    assert_eq!(payload.id, project_id);
    assert_eq!(payload.code, "coffee-legacy");
    assert_eq!(payload.name, "Coffee Legacy");
    assert_eq!(payload.description.as_deref(), Some("Retail edge rollout"));
    assert_eq!(payload.current_user_role, "admin");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_project_returns_not_found_for_unknown_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/999999")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
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
async fn update_project_updates_existing_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let row = sqlx::query(
        "INSERT INTO projects (code, name, description, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'Retail edge rollout', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let project_id: i64 = row.get("id");
    grant_admin_membership(&pool, project_id).await?;

    let cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"coffee-retail","name":"Coffee Retail","description":"Updated rollout","status":"archived"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ProjectSummary = read_json(response).await?;
    assert_eq!(payload.code, "coffee-retail");
    assert_eq!(payload.status, "archived");
    assert_eq!(payload.current_user_role, "admin");

    let row = sqlx::query("SELECT code, name, description, status FROM projects WHERE id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.get::<String, _>("code"), "coffee-retail");
    assert_eq!(row.get::<String, _>("name"), "Coffee Retail");
    assert_eq!(
        row.get::<Option<String>, _>("description").as_deref(),
        Some("Updated rollout")
    );
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn update_project_returns_not_found_for_unknown_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/projects/999999")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"coffee-legacy","name":"Coffee Legacy","status":"active"}"#,
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
async fn project_crud_flow_persists_changes_across_endpoints() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let admin_user_id = admin_user_id(&pool).await?;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"coffee-legacy","name":"Coffee Legacy","description":"Retail edge rollout","initial_admin_user_id":{admin_user_id}}}"#,
                )))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: CreatePlatformProjectResponse = read_json(create_response).await?;

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: ProjectListResponse = read_json(list_response).await?;
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id, created.project.id);

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}", created.project.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: ProjectSummary = read_json(detail_response).await?;
    assert_eq!(detail.code, "coffee-legacy");

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{}", created.project.id))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"coffee-retail","name":"Coffee Retail","description":"Updated rollout","status":"archived"}"#,
                ))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: ProjectSummary = read_json(update_response).await?;
    assert_eq!(updated.code, "coffee-retail");
    assert_eq!(updated.status, "archived");

    let detail_after_update = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}", created.project.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_after_update.status(), StatusCode::OK);
    let detail_after_update: ProjectSummary = read_json(detail_after_update).await?;
    assert_eq!(detail_after_update.code, "coffee-retail");
    assert_eq!(detail_after_update.name, "Coffee Retail");
    assert_eq!(detail_after_update.status, "archived");

    let row = sqlx::query("SELECT code, status FROM projects WHERE id = $1")
        .bind(created.project.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.get::<String, _>("code"), "coffee-retail");
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_project_alias_does_not_grant_platform_admin_project_visibility() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let password_hash = server::auth::hash_password("alice1234")
        .map_err(|error| std::io::Error::other(error.into_body().message))?;
    let alice_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, password_hash, status)
        VALUES ('alice', $1, 'active')
        RETURNING id
        "#,
    )
    .bind(password_hash)
    .fetch_one(&pool)
    .await?;

    let admin_cookie = login(&app).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"coffee-hidden","name":"Coffee Hidden","initial_admin_user_id":{alice_id}}}"#,
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    let admin_projects_response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(admin_projects_response.status(), StatusCode::OK);
    let admin_projects: ProjectListResponse = read_json(admin_projects_response).await?;
    assert!(admin_projects.items.is_empty());

    teardown(&database_url, &schema, pool).await
}
