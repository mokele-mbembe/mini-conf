use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    auth::AuthSessionResponse,
    draft::DraftResponse,
    release::{ReleaseDetailResponse, ReleaseDiffResponse, ReleaseListResponse, ReleaseSummary},
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
    let Some(database_url) = test_database_url("releases") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_releases");
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

async fn seed_project_config_deployment(pool: &PgPool) -> TestResult<(i64, i64, i64)> {
    seed_project_config_deployment_with_format(pool, "yaml", "normal", None).await
}

async fn seed_project_config_deployment_with_format(
    pool: &PgPool,
    format: &str,
    sensitivity: &str,
    secret_paths: Option<&serde_json::Value>,
) -> TestResult<(i64, i64, i64)> {
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
            sensitivity,
            secret_paths,
            status
        )
        VALUES ($1, 'main', 'Main Config', $2, $3, $4, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(format)
    .bind(sensitivity)
    .bind(secret_paths)
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
        VALUES ($1, 'prod', $2, $3, true, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(deployment_key)
    .bind(format!("{deployment_key} template"))
    .fetch_one(pool)
    .await?;

    Ok(deployment_id)
}

async fn seed_required_config_file(pool: &PgPool, project_id: i64, code: &str) -> TestResult<i64> {
    let config_file_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            is_required,
            format,
            sensitivity,
            status
        )
        VALUES ($1, $2, $3, true, 'yaml', 'normal', 'active')
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

async fn save_draft(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
    config_file_id: i64,
    content: &str,
    format: &str,
    base_version: Option<i64>,
) -> TestResult<DraftResponse> {
    let body = if let Some(base_version) = base_version {
        format!(
            r#"{{"content":{},"format":{},"base_version":{base_version}}}"#,
            serde_json::to_string(content)?,
            serde_json::to_string(format)?
        )
    } else {
        format!(
            r#"{{"content":{},"format":{}}}"#,
            serde_json::to_string(content)?,
            serde_json::to_string(format)?
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

async fn publish_release(
    app: &axum::Router,
    cookie: &str,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
    change_summary: Option<&str>,
) -> TestResult<ReleaseSummary> {
    let body = if let Some(change_summary) = change_summary {
        format!(
            r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id},"change_summary":{}}}"#,
            serde_json::to_string(change_summary)?
        )
    } else {
        format!(
            r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id}}}"#
        )
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
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
        "yaml",
        None,
    )
    .await?;

    let payload = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("increase polling interval"),
    )
    .await?;
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
        "SELECT revision, content, change_summary, diff_summary::text AS diff_summary, published_by FROM releases WHERE id = $1",
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
    let diff_summary: serde_json::Value =
        serde_json::from_str(&row.get::<String, _>("diff_summary"))?;
    assert_eq!(
        diff_summary,
        serde_json::json!({
            "is_initial": true,
            "has_changes": true,
            "added_lines": 1,
            "removed_lines": 0
        })
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
        "yaml",
        None,
    )
    .await?;

    let first = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        None,
    )
    .await?;

    let second = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        None,
    )
    .await?;

    assert_ne!(first.revision, second.revision);
    assert_eq!(first.content_hash, second.content_hash);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_succeeds_for_template_clone_target() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('alpha-clone-release', 'Alpha Clone Release', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let config_file_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            is_required,
            format,
            sensitivity,
            status
        )
        VALUES ($1, 'main', 'Main Config', true, 'yaml', 'normal', 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;
    let template_id = seed_template_deployment(&pool, project_id, "alpha-template").await?;

    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        template_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;

    let clone_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/deployment-instances/{template_id}/clone"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"deployment_key":"alpha-store-001","name":"Alpha Store 001","environment":"prod","clone_source":"draft"}"#,
                ))?,
        )
        .await?;
    assert_eq!(clone_response.status(), StatusCode::CREATED);
    let cloned: serde_json::Value = read_json(clone_response).await?;
    let deployment_id = cloned
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| std::io::Error::other("clone response should include deployment id"))?;

    let publish_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id},"change_summary":"clone publish"}}"#
                )))?,
        )
        .await?;

    assert_eq!(publish_response.status(), StatusCode::CREATED);
    let payload: ReleaseSummary = read_json(publish_response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert_eq!(payload.config_file_id, config_file_id);

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

#[tokio::test]
async fn list_releases_filters_by_deployment_instance() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let second_deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment,
            deployment_key,
            name,
            is_template,
            status
        )
        VALUES ($1, 'prod', 'store-002', 'Store 002', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;
    let cookie = login(&app).await?;

    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;
    let _ = save_draft(
        &app,
        &cookie,
        second_deployment_id,
        config_file_id,
        "poll_interval_ms: 8000\n",
        "yaml",
        None,
    )
    .await?;
    let _ = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("first"),
    )
    .await?;
    let target = publish_release(
        &app,
        &cookie,
        project_id,
        second_deployment_id,
        config_file_id,
        Some("second"),
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/releases?deployment_instance_id={second_deployment_id}"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ReleaseListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].id, target.id);
    assert_eq!(
        payload.items[0].deployment_instance_id,
        second_deployment_id
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_detail_returns_content_and_metadata() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;
    let created = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("increase polling interval"),
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}", created.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ReleaseDetailResponse = read_json(response).await?;
    assert_eq!(payload.release.id, created.id);
    assert_eq!(payload.content, "poll_interval_ms: 5000\n");
    assert!(!payload.content_redacted);
    assert_eq!(
        payload.diff_summary,
        Some(schema::release::ReleaseDiffSummary {
            is_initial: true,
            has_changes: true,
            added_lines: 1,
            removed_lines: 0,
        })
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_diff_requires_session_cookie() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/releases/1/diff")
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
async fn get_release_diff_returns_not_found_for_unknown_release() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/releases/999999/diff")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "release_not_found".to_owned(),
            message: "release not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_diff_returns_initial_release_shape_for_first_publish() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;
    let created = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("initial publish"),
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}/diff", created.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ReleaseDiffResponse = read_json(response).await?;
    assert_eq!(payload.release.id, created.id);
    assert!(payload.base_release.is_none());
    assert!(payload.before_content.is_none());
    assert_eq!(payload.after_content, "poll_interval_ms: 5000\n");
    assert!(payload.diff_summary.is_initial);
    assert!(payload.diff_summary.has_changes);
    assert_eq!(payload.diff_summary.added_lines, 1);
    assert_eq!(payload.diff_summary.removed_lines, 0);
    assert!(!payload.before_redacted);
    assert!(!payload.after_redacted);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_diff_returns_previous_release_content_and_summary() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 3000\nmode: steady\n",
        "yaml",
        None,
    )
    .await?;
    let first = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("initial publish"),
    )
    .await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\nmode: steady\nfeature_flag: true\n",
        "yaml",
        Some(1),
    )
    .await?;
    let second = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("bump interval"),
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}/diff", second.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ReleaseDiffResponse = read_json(response).await?;
    assert_eq!(payload.release.id, second.id);
    let Some(base_release) = payload.base_release else {
        return Err("base release should exist".into());
    };
    assert_eq!(base_release.id, first.id);
    assert_eq!(
        payload.before_content.as_deref(),
        Some("poll_interval_ms: 3000\nmode: steady\n")
    );
    assert_eq!(
        payload.after_content,
        "poll_interval_ms: 5000\nmode: steady\nfeature_flag: true\n"
    );
    assert!(!payload.diff_summary.is_initial);
    assert!(payload.diff_summary.has_changes);
    assert_eq!(payload.diff_summary.added_lines, 2);
    assert_eq!(payload.diff_summary.removed_lines, 1);
    assert!(!payload.before_redacted);
    assert!(!payload.after_redacted);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_diff_marks_identical_republish_as_no_change() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;
    let first = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        None,
    )
    .await?;
    let second = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        None,
    )
    .await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}/diff", second.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ReleaseDiffResponse = read_json(response).await?;
    let Some(base_release) = payload.base_release else {
        return Err("base release should exist".into());
    };
    assert_eq!(base_release.id, first.id);
    assert_eq!(
        payload.before_content.as_deref(),
        Some("poll_interval_ms: 5000\n")
    );
    assert_eq!(payload.after_content, "poll_interval_ms: 5000\n");
    assert!(!payload.diff_summary.is_initial);
    assert!(!payload.diff_summary.has_changes);
    assert_eq!(payload.diff_summary.added_lines, 0);
    assert_eq!(payload.diff_summary.removed_lines, 0);
    assert!(!payload.before_redacted);
    assert!(!payload.after_redacted);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn get_release_detail_returns_not_found_for_unknown_release() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/releases/999999")
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "release_not_found".to_owned(),
            message: "release not found".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_rejects_template_deployment_instances() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    sqlx::query("UPDATE deployment_instances SET is_template = TRUE WHERE id = $1")
        .bind(deployment_id)
        .execute(&pool)
        .await?;

    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;

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

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "deployment_instance_template_publish_forbidden".to_owned(),
            message: "template deployment instances cannot publish releases".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_rejects_when_required_config_is_missing() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let _required_config_file_id = seed_required_config_file(&pool, project_id, "vision").await?;
    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;

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

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "required_config_missing".to_owned(),
            message: "deployment instance is missing a required config".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_allows_required_config_when_it_has_existing_release() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let required_config_file_id = seed_required_config_file(&pool, project_id, "vision").await?;
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
        VALUES ($1, $2, $3, '20260407.0100', 'vision_enabled: true\n', repeat('d', 64), 'yaml', NULL, NULL, 'soft', $4)
        "#,
    )
    .bind(project_id)
    .bind(required_config_file_id)
    .bind(deployment_id)
    .bind(publisher_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms: 5000\n",
        "yaml",
        None,
    )
    .await?;

    let payload = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        None,
    )
    .await?;
    assert_eq!(payload.deployment_instance_id, deployment_id);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_rejects_invalid_yaml_in_existing_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let admin_user_id: i64 =
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
        VALUES ($1, $2, $3, $4, 'abc123', 'yaml', 1, $5)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind("poll_interval_ms: [\n")
    .bind(admin_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "draft_validation_failed");
    assert_eq!(payload.message, "draft content is not valid yaml");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn secret_release_detail_and_diff_are_redacted_for_management_reads() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    sqlx::query(
        r#"
        UPDATE config_files
        SET sensitivity = 'secret',
            secret_paths = '["$.wifi.password"]'::jsonb
        WHERE id = $1
        "#,
    )
    .bind(config_file_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "wifi:\n  password: secret-1\n",
        "yaml",
        None,
    )
    .await?;
    let first = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("initial"),
    )
    .await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "wifi:\n  password: secret-2\n",
        "yaml",
        Some(1),
    )
    .await?;
    let second = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("rotate"),
    )
    .await?;

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}", second.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: ReleaseDetailResponse = read_json(detail_response).await?;
    assert!(detail.content_redacted);
    assert!(detail.content.contains("***REDACTED***"));
    assert!(!detail.content.contains("secret-2"));

    let diff_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}/diff", second.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(diff_response.status(), StatusCode::OK);
    let diff: ReleaseDiffResponse = read_json(diff_response).await?;
    assert_eq!(diff.base_release.map(|release| release.id), Some(first.id));
    assert!(diff.before_redacted);
    assert!(diff.after_redacted);
    assert!(
        diff.before_content
            .as_deref()
            .is_some_and(|content| content.contains("***REDACTED***"))
    );
    assert!(diff.after_content.contains("***REDACTED***"));
    assert!(!diff.after_content.contains("secret-2"));

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_accepts_valid_toml_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) =
        seed_project_config_deployment_with_format(&pool, "toml", "normal", None).await?;
    let cookie = login(&app).await?;
    let _draft = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "poll_interval_ms = 5000\n",
        "toml",
        None,
    )
    .await?;

    let payload = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("publish toml"),
    )
    .await?;

    assert_eq!(payload.format, "toml");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn publish_release_rejects_invalid_toml_in_existing_draft() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (project_id, config_file_id, deployment_id) =
        seed_project_config_deployment_with_format(&pool, "toml", "normal", None).await?;
    let admin_user_id: i64 =
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
        VALUES ($1, $2, $3, $4, 'abc123', 'toml', 1, $5)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind("poll_interval_ms = ")
    .bind(admin_user_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
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

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "draft_validation_failed");
    assert_eq!(payload.message, "draft content is not valid toml");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn secret_toml_release_detail_and_diff_are_redacted_for_management_reads() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let secret_paths = serde_json::json!(["$.wifi.password"]);
    let (project_id, config_file_id, deployment_id) =
        seed_project_config_deployment_with_format(&pool, "toml", "secret", Some(&secret_paths))
            .await?;

    let cookie = login(&app).await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "[wifi]\npassword = \"secret-1\"\n",
        "toml",
        None,
    )
    .await?;
    let first = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("initial"),
    )
    .await?;
    let _ = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "[wifi]\npassword = \"secret-2\"\n",
        "toml",
        Some(1),
    )
    .await?;
    let second = publish_release(
        &app,
        &cookie,
        project_id,
        deployment_id,
        config_file_id,
        Some("rotate"),
    )
    .await?;

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}", second.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: ReleaseDetailResponse = read_json(detail_response).await?;
    assert!(detail.content_redacted);
    assert!(detail.content.contains("***REDACTED***"));
    assert!(!detail.content.contains("secret-2"));

    let diff_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases/{}/diff", second.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(diff_response.status(), StatusCode::OK);
    let diff: ReleaseDiffResponse = read_json(diff_response).await?;
    assert_eq!(diff.base_release.map(|release| release.id), Some(first.id));
    assert!(diff.before_redacted);
    assert!(diff.after_redacted);
    assert!(
        diff.before_content
            .as_deref()
            .is_some_and(|content| content.contains("***REDACTED***"))
    );
    assert!(diff.after_content.contains("***REDACTED***"));
    assert!(!diff.after_content.contains("secret-2"));

    teardown(&database_url, &schema, pool).await
}
