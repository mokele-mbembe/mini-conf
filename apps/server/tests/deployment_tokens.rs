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
        "INSERT INTO config_files (project_id, code, name, format) VALUES ($1, 'main', 'Main', 'yaml') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let environment_id: i64 = sqlx::query_scalar(
        "INSERT INTO project_environments (project_id, code, name, status, sort_order) VALUES ($1, 'prod', 'Production', 'active', 10) RETURNING id",
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    let deployment_id: i64 = sqlx::query_scalar(
        "INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name) VALUES ($1, $2, 'store-001', 'Store 001') RETURNING id",
    )
    .bind(project_id)
    .bind(environment_id)
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

async fn activate_deployment(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
) -> TestResult<axum::response::Response> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/activate"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    Ok(response)
}

async fn deactivate_deployment(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
) -> TestResult<axum::response::Response> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/deactivate"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    Ok(response)
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
async fn reset_token_rejects_inactive_deployment() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_, _, deployment_id) = seed_open_access_fixture(&pool, Some(OLD_TOKEN)).await?;
    sqlx::query("UPDATE deployment_instances SET status = 'inactive' WHERE id = $1")
        .bind(deployment_id)
        .execute(&pool)
        .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/token/reset"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "deployment_instance_inactive");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn activate_deployment_issues_token_and_enables_open_access() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, None).await?;
    sqlx::query("UPDATE deployment_instances SET status = 'inactive' WHERE id = $1")
        .bind(deployment_id)
        .execute(&pool)
        .await?;

    let cookie = login(&app).await?;
    let inactive_response = resolve_main_config(&app, OLD_TOKEN).await?;
    assert_eq!(inactive_response.status(), StatusCode::UNAUTHORIZED);

    let response = activate_deployment(&app, &cookie, deployment_id).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentTokenResetResponse = read_json(response).await?;
    assert_eq!(payload.deployment_instance_id, deployment_id);
    assert!(payload.token.starts_with("mc_live_"));

    let active_response = resolve_main_config(&app, &payload.token).await?;
    assert_eq!(active_response.status(), StatusCode::OK);
    let resolved: ResolveConfigResponse = read_json(active_response).await?;
    assert_eq!(resolved.release.revision, "20260409.0001");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn deactivate_deployment_invalidates_default_token() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, Some(OLD_TOKEN)).await?;
    let cookie = login(&app).await?;

    let active_response = resolve_main_config(&app, OLD_TOKEN).await?;
    assert_eq!(active_response.status(), StatusCode::OK);

    let response = deactivate_deployment(&app, &cookie, deployment_id).await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let row = sqlx::query(
        "SELECT d.status AS deployment_status, c.status AS credential_status FROM deployment_instances d JOIN deployment_credentials c ON c.deployment_instance_id = d.id WHERE d.id = $1 AND c.credential_name = 'default'",
    )
    .bind(deployment_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("deployment_status"), "inactive");
    assert_eq!(row.get::<String, _>("credential_status"), "inactive");

    let inactive_response = resolve_main_config(&app, OLD_TOKEN).await?;
    assert_eq!(inactive_response.status(), StatusCode::UNAUTHORIZED);
    let error: ErrorResponse = read_json(inactive_response).await?;
    assert_eq!(error.code, "invalid_token");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn activate_deployment_rejects_template() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let (_project_id, _config_file_id, deployment_id) =
        seed_open_access_fixture(&pool, None).await?;
    sqlx::query(
        "UPDATE deployment_instances SET is_template = TRUE, status = 'inactive' WHERE id = $1",
    )
    .bind(deployment_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = activate_deployment(&app, &cookie, deployment_id).await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload.code,
        "deployment_instance_template_activate_forbidden"
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
