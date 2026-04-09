#![allow(dead_code)]

use axum::{
    body::{Body, to_bytes},
    http::{Request, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::auth::AuthSessionResponse;
use server::{bootstrap, config::AppConfig};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use tower::util::ServiceExt;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub async fn setup_app(
    test_name: &str,
) -> TestResult<Option<(axum::Router, PgPool, String, String)>> {
    let Some(database_url) = test_database_url(test_name) else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_members");
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

    let pool = state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;
    install_admin_project_membership_trigger(&pool).await?;

    Ok(Some((server::app(state), pool, database_url, schema)))
}

pub async fn teardown(database_url: &str, schema: &str, pool: PgPool) -> TestResult {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url).await?;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;
    Ok(())
}

pub async fn read_json<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> TestResult<T> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(serde_json::from_slice(&body)?)
}

pub async fn login_as(app: &axum::Router, username: &str, password: &str) -> TestResult<String> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"username":{},"password":{}}}"#,
                    serde_json::to_string(username)?,
                    serde_json::to_string(password)?,
                )))?,
        )
        .await?;

    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .ok_or_else(|| std::io::Error::other("set-cookie should contain a session cookie"))?
        .to_owned();

    let _: AuthSessionResponse = read_json(response).await?;
    Ok(cookie)
}

pub async fn seed_user(pool: &PgPool, username: &str, password: &str) -> TestResult<i64> {
    let password_hash = server::auth::hash_password(password)
        .map_err(|error| std::io::Error::other(error.into_body().message))?;
    let user_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, password_hash, status)
        VALUES ($1, $2, 'active')
        RETURNING id
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;
    Ok(user_id)
}

pub async fn grant_project_role(
    pool: &PgPool,
    project_id: i64,
    username: &str,
    role: &str,
) -> TestResult<i64> {
    let user_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = $1 LIMIT 1")
        .bind(username)
        .fetch_one(pool)
        .await?;
    let member_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, user_id)
        DO UPDATE SET role = EXCLUDED.role
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await?;
    Ok(member_id)
}

pub async fn seed_config_file(pool: &PgPool, project_id: i64, code: &str) -> TestResult<i64> {
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

pub async fn seed_deployment_instance(
    pool: &PgPool,
    project_id: i64,
    deployment_key: &str,
    is_template: bool,
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
        VALUES ($1, 'prod', $2, $3, $4, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(deployment_key)
    .bind(format!("{deployment_key} deployment"))
    .bind(is_template)
    .fetch_one(pool)
    .await?;
    Ok(deployment_id)
}

pub async fn seed_release(
    pool: &PgPool,
    project_id: i64,
    config_file_id: i64,
    deployment_id: i64,
    revision: &str,
) -> TestResult<i64> {
    let release_id: i64 = sqlx::query_scalar(
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
        VALUES ($1, $2, $3, $4, 'log_level: info\n', 'abc123', 'yaml', 'initial', '{"is_initial":true,"has_changes":true,"added_lines":1,"removed_lines":0}', 'soft', 1)
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(revision)
    .fetch_one(pool)
    .await?;
    Ok(release_id)
}

pub async fn seed_sync_record(
    pool: &PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
    release_id: i64,
) -> TestResult {
    sqlx::query(
        r#"
        INSERT INTO deployment_sync_records (
            project_id,
            deployment_instance_id,
            config_file_id,
            release_id,
            process_key,
            revision,
            action,
            status,
            message,
            detail,
            reported_at
        )
        VALUES ($1, $2, $3, $4, 'main', '20260410.0001', 'apply', 'success', 'config applied', '{"duration_ms":87}', '2026-04-10T12:00:00Z')
        "#,
    )
    .bind(project_id)
    .bind(deployment_id)
    .bind(config_file_id)
    .bind(release_id)
    .execute(pool)
    .await?;
    Ok(())
}

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
