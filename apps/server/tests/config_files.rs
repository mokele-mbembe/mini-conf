use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    auth::AuthSessionResponse,
    config_file::{ConfigFileListResponse, ConfigFileSummary},
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
    let Some(database_url) = test_database_url("config files") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_config_files");
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

#[tokio::test]
async fn list_config_files_filters_by_project() -> TestResult {
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
        INSERT INTO config_files (project_id, code, name, format, sensitivity, status)
        VALUES
            ($1, 'main', 'Main', 'yaml', 'normal', 'active'),
            ($1, 'vision', 'Vision', 'yaml', 'secret', 'active'),
            ($2, 'main', 'Main', 'json', 'normal', 'active')
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
                .uri(format!("/api/config-files?project_id={project_a}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ConfigFileListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 2);
    assert!(
        payload
            .items
            .iter()
            .all(|item| item.project_id == project_a)
    );
    assert_eq!(payload.items[0].code, "main");
    assert_eq!(payload.items[1].code, "vision");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_config_file_creates_row_with_secret_paths() -> TestResult {
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
                .uri("/api/config-files")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"code":"main","name":"Main Config","is_required":true,"format":"yaml","schema_name":"coffee-main","schema_version":"v1","sensitivity":"secret","secret_paths":["$.wifi.password","$.third_party.api_key"],"description":"Primary device configuration"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: ConfigFileSummary = read_json(response).await?;
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.code, "main");
    assert!(payload.is_required);
    assert_eq!(payload.sensitivity, "secret");
    assert_eq!(
        payload.secret_paths.as_deref(),
        Some(
            &[
                "$.wifi.password".to_owned(),
                "$.third_party.api_key".to_owned()
            ][..]
        )
    );

    let row = sqlx::query(
        "SELECT code, is_required, schema_name, schema_version, sensitivity, secret_paths FROM config_files WHERE id = $1",
    )
    .bind(payload.id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("code"), "main");
    assert!(row.get::<bool, _>("is_required"));
    assert_eq!(
        row.get::<Option<String>, _>("schema_name").as_deref(),
        Some("coffee-main")
    );
    assert_eq!(
        row.get::<Option<String>, _>("schema_version").as_deref(),
        Some("v1")
    );
    assert_eq!(row.get::<String, _>("sensitivity"), "secret");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_config_file_rejects_duplicate_code_within_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO config_files (project_id, code, name, format, sensitivity, status) VALUES ($1, 'main', 'Main', 'yaml', 'normal', 'active')",
    )
    .bind(project_id)
    .execute(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config-files")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"code":"main","name":"Main Duplicate","format":"yaml"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "config_file_code_conflict".to_owned(),
            message: "config file code already exists in project".to_owned(),
        }
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_config_file_returns_project_not_found_for_unknown_project() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/config-files")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"project_id":999999,"code":"main","name":"Main Config","format":"yaml"}"#,
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
async fn get_config_file_returns_detail_for_authenticated_session() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format, sensitivity, status) VALUES ($1, 'main', 'Main Config', 'yaml', 'normal', 'active') RETURNING id",
    )
    .bind(project_id)
    .fetch_one(&pool)
    .await?;

    let cookie = login(&app).await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/config-files/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ConfigFileSummary = read_json(response).await?;
    assert_eq!(payload.id, config_file_id);
    assert_eq!(payload.project_id, project_id);
    assert_eq!(payload.code, "main");
    assert!(!payload.is_required);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn update_config_file_updates_existing_row() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app().await? else {
        return Ok(());
    };

    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-legacy', 'Coffee Legacy', 'active') RETURNING id",
    )
    .fetch_one(&pool)
    .await?;
    let config_file_id: i64 = sqlx::query_scalar(
        "INSERT INTO config_files (project_id, code, name, format, sensitivity, status) VALUES ($1, 'main', 'Main Config', 'yaml', 'normal', 'active') RETURNING id",
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
                .uri(format!("/api/config-files/{config_file_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"code":"main-v2","name":"Main Config V2","is_required":true,"format":"json","schema_name":"coffee-main","schema_version":"v2","sensitivity":"secret","secret_paths":["$.wifi.password"],"description":"Updated config","status":"archived"}}"#
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ConfigFileSummary = read_json(response).await?;
    assert_eq!(payload.code, "main-v2");
    assert!(payload.is_required);
    assert_eq!(payload.format, "json");
    assert_eq!(payload.status, "archived");

    let row = sqlx::query(
        "SELECT code, is_required, format, schema_version, sensitivity, status FROM config_files WHERE id = $1",
    )
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(row.get::<String, _>("code"), "main-v2");
    assert!(row.get::<bool, _>("is_required"));
    assert_eq!(row.get::<String, _>("format"), "json");
    assert_eq!(
        row.get::<Option<String>, _>("schema_version").as_deref(),
        Some("v2")
    );
    assert_eq!(row.get::<String, _>("sensitivity"), "secret");
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn config_file_crud_flow_persists_changes_across_endpoints() -> TestResult {
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
                .uri("/api/config-files")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"code":"main","name":"Main Config","is_required":false,"format":"yaml","description":"Primary config"}}"#
                )))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: ConfigFileSummary = read_json(create_response).await?;
    assert!(!created.is_required);

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/config-files?project_id={project_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: ConfigFileListResponse = read_json(list_response).await?;
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id, created.id);

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/config-files/{}", created.id))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: ConfigFileSummary = read_json(detail_response).await?;
    assert_eq!(detail.code, "main");

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/config-files/{}", created.id))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"code":"main-v2","name":"Main Config V2","is_required":true,"format":"json","sensitivity":"secret","secret_paths":["$.wifi.password"],"description":"Updated config","status":"archived"}}"#
                )))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: ConfigFileSummary = read_json(update_response).await?;
    assert_eq!(updated.code, "main-v2");
    assert!(updated.is_required);
    assert_eq!(updated.status, "archived");

    let detail_after_update = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/config-files/{}", created.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_after_update.status(), StatusCode::OK);
    let detail_after_update: ConfigFileSummary = read_json(detail_after_update).await?;
    assert_eq!(detail_after_update.code, "main-v2");
    assert!(detail_after_update.is_required);
    assert_eq!(detail_after_update.format, "json");
    assert_eq!(detail_after_update.status, "archived");

    let row = sqlx::query("SELECT code, is_required, status FROM config_files WHERE id = $1")
        .bind(created.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(row.get::<String, _>("code"), "main-v2");
    assert!(row.get::<bool, _>("is_required"));
    assert_eq!(row.get::<String, _>("status"), "archived");

    teardown(&database_url, &schema, pool).await
}
