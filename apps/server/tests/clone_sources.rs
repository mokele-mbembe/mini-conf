use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{auth::AuthSessionResponse, clone_source::CloneSourceListResponse};
use server::{bootstrap, config::AppConfig};
use sqlx::{Connection, Executor, PgConnection, PgPool};
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
    let Some(database_url) = test_database_url("clone_sources") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_clone_sources");
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

/// Creates project + environment + config_file + target deployment.
/// Returns (project_id, config_file_id, target_deployment_id, environment_id).
async fn seed_base(pool: &PgPool) -> TestResult<(i64, i64, i64, i64)> {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('cs-project', 'CS Project', 'active') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let environment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO project_environments (project_id, code, name, status, sort_order)
        VALUES ($1, 'prod', 'Production', 'active', 10)
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let config_file_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (project_id, code, name, format, sensitivity, status)
        VALUES ($1, 'main', 'Main Config', 'yaml', 'normal', 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let target_deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'target-001', 'Target 001', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await?;

    Ok((
        project_id,
        config_file_id,
        target_deployment_id,
        environment_id,
    ))
}

async fn seed_deployment(
    pool: &PgPool,
    project_id: i64,
    environment_id: i64,
    key: &str,
    name: &str,
    is_template: bool,
) -> TestResult<i64> {
    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, $3, $4, $5, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .bind(key)
    .bind(name)
    .bind(is_template)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

async fn seed_draft(
    pool: &PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
) -> TestResult {
    let user_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
        .fetch_one(pool)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO drafts (project_id, config_file_id, deployment_instance_id, content, content_hash, format, editor_user_id)
        VALUES ($1, $2, $3, 'key: value', repeat('a', 64), 'yaml', $4)
        ON CONFLICT (config_file_id, deployment_instance_id) DO NOTHING
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_release(
    pool: &PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
) -> TestResult {
    let user_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
        .fetch_one(pool)
        .await?;
    let revision = format!("20260420.{deployment_id:04}");
    sqlx::query(
        r#"
        INSERT INTO releases (project_id, config_file_id, deployment_instance_id, revision, content, content_hash, format, apply_mode, published_by)
        VALUES ($1, $2, $3, $4, 'key: value', repeat('b', 64), 'yaml', 'soft', $5)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(&revision)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

fn clone_sources_uri(project_id: i64, target_id: i64, config_file_id: i64) -> String {
    format!(
        "/api/clone-sources?project_id={project_id}&target_deployment_id={target_id}&config_file_id={config_file_id}"
    )
}

// ---------- Tests ----------

#[tokio::test]
async fn list_clone_sources_returns_empty_when_no_other_instances() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, _env_id) = seed_base(&pool).await?;
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(clone_sources_uri(project_id, target_id, config_file_id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: CloneSourceListResponse = read_json(response).await?;
    assert!(payload.items.is_empty());
    assert_eq!(payload.next_cursor, None);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_excludes_target_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, env_id) = seed_base(&pool).await?;
    let _other_id =
        seed_deployment(&pool, project_id, env_id, "other-001", "Other 001", false).await?;
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(clone_sources_uri(project_id, target_id, config_file_id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].deployment_key, "other-001");
    // Target itself should not appear
    assert!(
        payload
            .items
            .iter()
            .all(|i| i.deployment_instance_id != target_id)
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_returns_availability_metadata() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, env_id) = seed_base(&pool).await?;

    // Source A: has draft only
    let a_id = seed_deployment(&pool, project_id, env_id, "source-a", "Source A", false).await?;
    seed_draft(&pool, project_id, a_id, config_file_id).await?;

    // Source B: has release only
    let b_id = seed_deployment(&pool, project_id, env_id, "source-b", "Source B", false).await?;
    seed_release(&pool, project_id, b_id, config_file_id).await?;

    // Source C: has both draft and release
    let c_id = seed_deployment(&pool, project_id, env_id, "source-c", "Source C", false).await?;
    seed_draft(&pool, project_id, c_id, config_file_id).await?;
    seed_release(&pool, project_id, c_id, config_file_id).await?;

    // Source D: has neither
    let _d_id = seed_deployment(&pool, project_id, env_id, "source-d", "Source D", false).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(clone_sources_uri(project_id, target_id, config_file_id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 4);

    // Results ordered by id ASC, so a, b, c, d
    let a = payload
        .items
        .iter()
        .find(|i| i.deployment_key == "source-a")
        .ok_or_else(|| std::io::Error::other("expected source-a"))?;
    assert!(a.available_sources.draft);
    assert!(!a.available_sources.latest_release);

    let b = payload
        .items
        .iter()
        .find(|i| i.deployment_key == "source-b")
        .ok_or_else(|| std::io::Error::other("expected source-b"))?;
    assert!(!b.available_sources.draft);
    assert!(b.available_sources.latest_release);

    let c = payload
        .items
        .iter()
        .find(|i| i.deployment_key == "source-c")
        .ok_or_else(|| std::io::Error::other("expected source-c"))?;
    assert!(c.available_sources.draft);
    assert!(c.available_sources.latest_release);

    let d = payload
        .items
        .iter()
        .find(|i| i.deployment_key == "source-d")
        .ok_or_else(|| std::io::Error::other("expected source-d"))?;
    assert!(!d.available_sources.draft);
    assert!(!d.available_sources.latest_release);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_returns_template_flag() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, env_id) = seed_base(&pool).await?;
    let _tpl_id =
        seed_deployment(&pool, project_id, env_id, "tpl-001", "Template 001", true).await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(clone_sources_uri(project_id, target_id, config_file_id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert!(payload.items[0].is_template);
    assert_eq!(payload.items[0].environment_name, "Production");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_supports_keyword_search() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, env_id) = seed_base(&pool).await?;
    seed_deployment(&pool, project_id, env_id, "alpha-001", "Alpha Store", false).await?;
    seed_deployment(&pool, project_id, env_id, "beta-002", "Beta Store", false).await?;

    let cookie = login(&app).await?;
    let uri = format!(
        "{}&keyword=alpha",
        clone_sources_uri(project_id, target_id, config_file_id)
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri(&uri)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].deployment_key, "alpha-001");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_supports_cursor_pagination() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, env_id) = seed_base(&pool).await?;

    // Create 3 extra deployments so we can paginate with limit=2
    let _id1 = seed_deployment(&pool, project_id, env_id, "d-001", "Deploy 001", false).await?;
    let id2 = seed_deployment(&pool, project_id, env_id, "d-002", "Deploy 002", false).await?;
    let _id3 = seed_deployment(&pool, project_id, env_id, "d-003", "Deploy 003", false).await?;

    let cookie = login(&app).await?;

    // First page: limit=2
    let uri = format!(
        "{}&limit=2",
        clone_sources_uri(project_id, target_id, config_file_id)
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&uri)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let page1: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(page1.items.len(), 2);
    assert!(page1.next_cursor.is_some());
    assert_eq!(page1.next_cursor, Some(id2));
    let next_cursor = page1
        .next_cursor
        .ok_or_else(|| std::io::Error::other("expected next cursor"))?;

    // Second page: cursor=id2
    let uri = format!(
        "{}&limit=2&cursor={}",
        clone_sources_uri(project_id, target_id, config_file_id),
        next_cursor
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri(&uri)
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let page2: CloneSourceListResponse = read_json(response).await?;
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.next_cursor, None);

    // All items together should be 3 unique
    let all_keys: Vec<&str> = page1
        .items
        .iter()
        .chain(page2.items.iter())
        .map(|i| i.deployment_key.as_str())
        .collect();
    assert_eq!(all_keys.len(), 3);
    assert!(all_keys.contains(&"d-001"));
    assert!(all_keys.contains(&"d-002"));
    assert!(all_keys.contains(&"d-003"));

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn list_clone_sources_requires_auth() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, target_id, _env_id) = seed_base(&pool).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(clone_sources_uri(project_id, target_id, config_file_id))
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    teardown(&database_url, &schema, pool).await
}
