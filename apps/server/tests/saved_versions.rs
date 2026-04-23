use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use schema::{
    auth::AuthSessionResponse,
    draft::DraftResponse,
    saved_version::{
        SavedVersionDetailResponse, SavedVersionListResponse, SavedVersionRestoreResponse,
    },
};
use server::{bootstrap, config::AppConfig, error::ErrorResponse};
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
    sqlx::query("DROP TRIGGER IF EXISTS trg_auto_grant_test_admin_project_member ON projects;")
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
    let Some(database_url) = test_database_url("saved_versions") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_sv");
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
    serde_json::from_slice(&body).map_err(|error| {
        let body_text = String::from_utf8_lossy(&body).into_owned();
        std::io::Error::other(format!("JSON decode failed: {error}; body={body_text}")).into()
    })
}

fn session_cookie(response: &axum::response::Response) -> TestResult<String> {
    let header_value = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or("set-cookie header should exist")?;
    let value = header_value.to_str()?;
    let cookie = value
        .split(';')
        .next()
        .ok_or("set-cookie should contain a session cookie")?;
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
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ('coffee-sv', 'Coffee SV', 'active') RETURNING id",
    )
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
    let deployment_id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (project_id, environment_id, deployment_key, name, is_template, status)
        VALUES ($1, $2, 'store-001', 'Store 001', false, 'active')
        RETURNING id
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_one(pool)
    .await?;

    Ok((project_id, config_file_id, deployment_id))
}

async fn save_draft(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
    config_file_id: i64,
    content: &str,
    base_version: Option<i64>,
) -> TestResult<DraftResponse> {
    let bv = match base_version {
        Some(v) => format!(r#","base_version":{v}"#),
        None => String::new(),
    };
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(format!(
                    r#"{{"content":{content_json},"format":"yaml"{bv}}}"#,
                    content_json = serde_json::to_string(content)?,
                )))?,
        )
        .await?;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "save_draft should succeed"
    );
    read_json(response).await
}

async fn list_saved_versions(
    app: &axum::Router,
    cookie: &str,
    deployment_id: i64,
    config_file_id: i64,
) -> TestResult<SavedVersionListResponse> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/api/draft-saved-versions?deployment_instance_id={deployment_id}&config_file_id={config_file_id}"
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    read_json(response).await
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn list_saved_versions_returns_empty_list() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    assert!(list.items.is_empty());

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn saving_draft_auto_generates_saved_version() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    let draft = save_draft(&app, &cookie, deployment_id, config_file_id, "a: 1\n", None).await?;
    assert_eq!(draft.version, 1);

    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].source_draft_version, 1);

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn duplicate_content_does_not_create_duplicate_saved_version() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    let draft = save_draft(&app, &cookie, deployment_id, config_file_id, "a: 1\n", None).await?;
    // Save again with same content
    let _draft2 = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "a: 1\n",
        Some(draft.version),
    )
    .await?;

    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    assert_eq!(
        list.items.len(),
        1,
        "duplicate content should not create a new saved version"
    );

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn get_saved_version_returns_detail_with_content() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    save_draft(&app, &cookie, deployment_id, config_file_id, "b: 2\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let detail: SavedVersionDetailResponse = read_json(response).await?;
    assert_eq!(detail.saved_version.content, "b: 2\n");
    assert_eq!(detail.saved_version.format, "yaml");

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn patch_note_succeeds() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    save_draft(&app, &cookie, deployment_id, config_file_id, "c: 3\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"note":"调试参数"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let detail: SavedVersionDetailResponse = read_json(response).await?;
    assert_eq!(detail.saved_version.note.as_deref(), Some("调试参数"));

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn patch_note_too_long_returns_422() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    save_draft(&app, &cookie, deployment_id, config_file_id, "d: 4\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    let long_note = "x".repeat(501);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(r#"{{"note":"{long_note}"}}"#)))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let err: ErrorResponse = read_json(response).await?;
    assert_eq!(err.code, "saved_version_note_too_long");

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn restore_saved_version_overwrites_current_draft() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    // Save v1
    let draft_v1 = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "v1: true\n",
        None,
    )
    .await?;
    // Save v2
    let draft_v2 = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "v2: true\n",
        Some(draft_v1.version),
    )
    .await?;

    // Get the saved version for v1
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    // List is ordered by created_at DESC, so v2 is first, v1 is second
    assert_eq!(list.items.len(), 2);
    let sv_v1_id = list.items[1].id; // older one

    // Restore v1
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/draft-saved-versions/{sv_v1_id}/restore"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(
                    r#"{{"base_version":{}}}"#,
                    draft_v2.version
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let restore: SavedVersionRestoreResponse = read_json(response).await?;
    assert_eq!(restore.draft.content, "v1: true\n");
    assert_eq!(restore.draft.version, draft_v2.version + 1);

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn restore_with_stale_base_version_returns_409() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    let draft = save_draft(&app, &cookie, deployment_id, config_file_id, "e: 5\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    // Use stale base_version
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/draft-saved-versions/{sv_id}/restore"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(
                    r#"{{"base_version":{}}}"#,
                    draft.version + 999
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let err: ErrorResponse = read_json(response).await?;
    assert_eq!(err.code, "draft_version_conflict");

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn delete_saved_version_removes_from_list() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    save_draft(&app, &cookie, deployment_id, config_file_id, "f: 6\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    assert!(list.items.is_empty());

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn delete_saved_version_does_not_affect_current_draft() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    save_draft(&app, &cookie, deployment_id, config_file_id, "g: 7\n", None).await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    // Delete saved version
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Draft should still be there
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let draft: DraftResponse = read_json(response).await?;
    assert_eq!(draft.content, "g: 7\n");

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn restore_does_not_affect_release_history() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;

    // Create a draft and publish it
    save_draft(&app, &cookie, deployment_id, config_file_id, "h: 8\n", None).await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/releases/publish")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(
                    r#"{{"project_id":{project_id},"deployment_instance_id":{deployment_id},"config_file_id":{config_file_id}}}"#,
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Count releases before restore
    let release_count_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM releases WHERE deployment_instance_id = $1 AND config_file_id = $2",
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;

    // Get the draft (published increments version)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    let current_draft: DraftResponse = read_json(response).await?;

    // Now save a different version
    let draft_v2 = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "h2: 82\n",
        Some(current_draft.version),
    )
    .await?;

    // Restore v1 from saved versions
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_v1_id = list
        .items
        .last()
        .ok_or("should have at least one saved version")?
        .id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/draft-saved-versions/{sv_v1_id}/restore"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(
                    r#"{{"base_version":{}}}"#,
                    draft_v2.version,
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    // Release count should be unchanged
    let release_count_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM releases WHERE deployment_instance_id = $1 AND config_file_id = $2",
    )
    .bind(deployment_id)
    .bind(config_file_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(release_count_before, release_count_after);

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn non_member_gets_empty_list() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;
    save_draft(&app, &cookie, deployment_id, config_file_id, "i: 9\n", None).await?;

    // Create a second user with no project membership
    let password_hash = server::auth::hash_password("viewer123456")
        .map_err(|e| std::io::Error::other(e.into_body().message))?;
    sqlx::query(
        "INSERT INTO users (username, password_hash, status) VALUES ('viewer2', $1, 'active')",
    )
    .bind(&password_hash)
    .execute(&pool)
    .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"viewer2","password":"viewer123456"}"#,
                ))?,
        )
        .await?;
    let viewer_cookie = session_cookie(&response)?;
    let _: AuthSessionResponse = read_json(response).await?;

    // Non-member listing returns empty
    let list = list_saved_versions(&app, &viewer_cookie, deployment_id, config_file_id).await?;
    assert!(list.items.is_empty());

    // Non-member detail returns 404
    let admin_list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = admin_list.items[0].id;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn viewer_cannot_restore_or_delete() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (project_id, config_file_id, deployment_id) = seed_project_config_deployment(&pool).await?;
    let draft = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "j: 10\n",
        None,
    )
    .await?;

    // Create a viewer user
    let password_hash = server::auth::hash_password("viewer123456")
        .map_err(|e| std::io::Error::other(e.into_body().message))?;
    sqlx::query(
        "INSERT INTO users (username, password_hash, status) VALUES ('viewer3', $1, 'active')",
    )
    .bind(&password_hash)
    .execute(&pool)
    .await?;
    let viewer_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'viewer3' LIMIT 1")
            .fetch_one(&pool)
            .await?;
    sqlx::query(
        "INSERT INTO project_members (project_id, user_id, role) VALUES ($1, $2, 'viewer')",
    )
    .bind(project_id)
    .bind(viewer_user_id)
    .execute(&pool)
    .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"viewer3","password":"viewer123456"}"#,
                ))?,
        )
        .await?;
    let viewer_cookie = session_cookie(&response)?;
    let _: AuthSessionResponse = read_json(response).await?;

    // Viewer cannot list saved versions
    let viewer_list =
        list_saved_versions(&app, &viewer_cookie, deployment_id, config_file_id).await?;
    assert!(
        viewer_list.items.is_empty(),
        "viewer should not see saved versions"
    );

    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    // Viewer cannot restore
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/draft-saved-versions/{sv_id}/restore"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::from(format!(
                    r#"{{"base_version":{}}}"#,
                    draft.version,
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Viewer cannot delete
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    teardown(&db_url, &schema, pool).await
}

#[tokio::test]
async fn audit_logs_are_written_for_saved_version_operations() -> TestResult {
    let Some((app, pool, db_url, schema)) = setup_app().await? else {
        return Ok(());
    };
    let cookie = login(&app).await?;
    let (_project_id, config_file_id, deployment_id) =
        seed_project_config_deployment(&pool).await?;

    // Save draft → creates saved_version.created
    let draft = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "k: 11\n",
        None,
    )
    .await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    let sv_id = list.items[0].id;

    // Patch note → saved_version.updated
    let _response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/draft-saved-versions/{sv_id}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(r#"{"note":"audit test"}"#))?,
        )
        .await?;

    // Restore → saved_version.restored
    let _response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/draft-saved-versions/{sv_id}/restore"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(format!(
                    r#"{{"base_version":{}}}"#,
                    draft.version,
                )))?,
        )
        .await?;

    // Delete → saved_version.deleted
    // Need a new saved version since we want to test delete audit log
    let draft2 = save_draft(
        &app,
        &cookie,
        deployment_id,
        config_file_id,
        "k2: 112\n",
        Some(draft.version + 1),
    )
    .await?;
    let list = list_saved_versions(&app, &cookie, deployment_id, config_file_id).await?;
    // Find one that's not the one we already used
    let sv_id_for_delete = list
        .items
        .iter()
        .find(|i| i.source_draft_version == draft2.version)
        .ok_or("should have a saved version for the second save")?
        .id;

    let _response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/draft-saved-versions/{sv_id_for_delete}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    // Verify audit logs
    let actions: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT action FROM audit_logs
        WHERE resource_type = 'saved_version'
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        actions.contains(&"saved_version.created".to_owned()),
        "should have created audit log, got: {actions:?}"
    );
    assert!(
        actions.contains(&"saved_version.updated".to_owned()),
        "should have updated audit log, got: {actions:?}"
    );
    assert!(
        actions.contains(&"saved_version.restored".to_owned()),
        "should have restored audit log, got: {actions:?}"
    );
    assert!(
        actions.contains(&"saved_version.deleted".to_owned()),
        "should have deleted audit log, got: {actions:?}"
    );

    teardown(&db_url, &schema, pool).await
}
