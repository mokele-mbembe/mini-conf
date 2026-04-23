#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::setup::SetupStatusResponse;
use server::error::ErrorResponse;
use support::{
    TestResult, login_as, lookup_user_id, read_json, seed_user, setup_app_pending_setup, teardown,
};
use tower::util::ServiceExt;

#[tokio::test]
async fn setup_status_defaults_to_required_with_seeded_platform_admin() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup status default").await?
    else {
        return Ok(());
    };

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/setup/status")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: SetupStatusResponse = read_json(response).await?;
    assert!(payload.setup_required);
    assert_eq!(payload.setup_completed_at, None);
    assert_eq!(payload.setup_completed_by_user_id, None);
    assert_eq!(payload.active_platform_admin_count, 1);
    assert_eq!(payload.project_count, 0);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn setup_status_reflects_completed_setup_marker() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup status complete").await?
    else {
        return Ok(());
    };

    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    sqlx::query(
        r#"
        UPDATE system_settings
        SET
            setup_completed_at = NOW(),
            setup_completed_by_user_id = $1,
            updated_at = NOW()
        WHERE id = 1
        "#,
    )
    .bind(admin_user_id)
    .execute(&pool)
    .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/setup/status")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: SetupStatusResponse = read_json(response).await?;
    assert!(!payload.setup_required);
    assert_eq!(payload.setup_completed_by_user_id, Some(admin_user_id));
    assert!(payload.setup_completed_at.is_some());
    assert_eq!(payload.active_platform_admin_count, 1);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn platform_admin_can_complete_setup() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup complete platform admin").await?
    else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/setup/complete")
                .header(header::COOKIE, admin_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: SetupStatusResponse = read_json(response).await?;
    assert!(!payload.setup_required);
    assert_eq!(payload.setup_completed_by_user_id, Some(admin_user_id));
    assert!(payload.setup_completed_at.is_some());

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn non_platform_admin_cannot_complete_setup() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup complete forbidden").await?
    else {
        return Ok(());
    };

    seed_user(&pool, "setup-viewer", "viewer12345").await?;
    let viewer_cookie = login_as(&app, "setup-viewer", "viewer12345").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/setup/complete")
                .header(header::COOKIE, viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "platform_permission_denied");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_endpoints_require_completed_setup() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup gate projects").await?
    else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&serde_json::json!({
                    "code": "setup-blocked",
                    "name": "Setup Blocked",
                    "description": "should be blocked before setup is complete",
                    "initial_admin_user_id": admin_user_id,
                }))?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "setup_required");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn setup_allows_platform_initialization_admin_apis() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup gate allows admin initialization").await?
    else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let create_user_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users")
                .header(header::COOKIE, admin_cookie.clone())
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"setup-project-admin","password":"TempPassword123!","status":"active","is_platform_admin":false,"must_change_password":true}"#,
                ))?,
        )
        .await?;

    assert_eq!(create_user_response.status(), StatusCode::CREATED);
    let created_user: serde_json::Value = read_json(create_user_response).await?;
    let Some(initial_admin_user_id) = created_user.get("id").and_then(|id| id.as_i64()) else {
        return Err(
            std::io::Error::other("created admin user response should include numeric id").into(),
        );
    };

    let create_project_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/projects")
                .header(header::COOKIE, admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&serde_json::json!({
                    "code": "setup-project",
                    "name": "Setup Project",
                    "description": "created before setup completion",
                    "initial_admin_user_id": initial_admin_user_id,
                }))?))?,
        )
        .await?;

    assert_eq!(create_project_response.status(), StatusCode::CREATED);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn completing_setup_unlocks_project_endpoints() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app_pending_setup("setup gate unlock").await?
    else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;

    let complete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/setup/complete")
                .header(header::COOKIE, admin_cookie.clone())
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(complete_response.status(), StatusCode::OK);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&serde_json::json!({
                    "code": "setup-unlocked",
                    "name": "Setup Unlocked",
                    "description": "allowed after setup completion",
                    "initial_admin_user_id": admin_user_id,
                }))?))?,
        )
        .await?;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    teardown(&database_url, &schema, pool).await
}
