#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::admin_user::AdminUserSummary;
use server::error::ErrorResponse;
use support::{TestResult, login_as, read_json, seed_user, setup_app, teardown};
use tower::util::ServiceExt;

#[tokio::test]
async fn platform_admin_can_create_user() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users create").await? else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users")
                .header(header::COOKIE, admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"alice","password":"TempPassword123!","is_platform_admin":false,"must_change_password":true,"status":"active"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload: AdminUserSummary = read_json(response).await?;
    assert_eq!(payload.username, "alice");
    assert_eq!(payload.status, "active");
    assert!(!payload.is_platform_admin);
    assert!(payload.must_change_password);
    assert_eq!(payload.project_count, 0);
    assert!(payload.password_updated_at.is_some());

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn non_platform_admin_cannot_create_user() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users forbidden").await? else {
        return Ok(());
    };

    seed_user(&pool, "operator", "operator123").await?;
    let operator_cookie = login_as(&app, "operator", "operator123").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users")
                .header(header::COOKIE, operator_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"alice","password":"TempPassword123!","is_platform_admin":false,"must_change_password":true,"status":"active"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "platform_permission_denied");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn disabled_user_cannot_login_after_platform_admin_update() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users disable login").await?
    else {
        return Ok(());
    };

    let user_id = seed_user(&pool, "alice", "alice1234").await?;
    let admin_cookie = login_as(&app, "admin", "admin123456").await?;

    let disable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{user_id}"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"disabled"}"#))?,
        )
        .await?;

    assert_eq!(disable_response.status(), StatusCode::OK);

    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"alice","password":"alice1234"}"#))?,
        )
        .await?;

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(login_response).await?;
    assert_eq!(payload.code, "invalid_credentials");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn disabling_user_revokes_existing_session_and_reenable_does_not_restore_it() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users disable revoke").await?
    else {
        return Ok(());
    };

    let user_id = seed_user(&pool, "alice", "alice1234").await?;
    let old_cookie = login_as(&app, "alice", "alice1234").await?;
    let admin_cookie = login_as(&app, "admin", "admin123456").await?;

    let disable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{user_id}"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"disabled"}"#))?,
        )
        .await?;
    assert_eq!(disable_response.status(), StatusCode::OK);

    let enable_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{user_id}"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"status":"active"}"#))?,
        )
        .await?;
    assert_eq!(enable_response.status(), StatusCode::OK);

    let me_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, old_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);
    let me_error: ErrorResponse = read_json(me_response).await?;
    assert_eq!(me_error.code, "auth_session_expired");

    let new_login_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"alice","password":"alice1234"}"#))?,
        )
        .await?;
    assert_eq!(new_login_response.status(), StatusCode::OK);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn reset_password_revokes_existing_sessions() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users reset session").await?
    else {
        return Ok(());
    };

    let user_id = seed_user(&pool, "alice", "alice1234").await?;
    let user_cookie = login_as(&app, "alice", "alice1234").await?;
    let admin_cookie = login_as(&app, "admin", "admin123456").await?;

    let reset_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/users/{user_id}/reset-password"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"new_password":"NewTempPassword123!","must_change_password":true}"#,
                ))?,
        )
        .await?;

    assert_eq!(reset_response.status(), StatusCode::NO_CONTENT);

    let me_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/me")
                .header(header::COOKIE, user_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(me_response.status(), StatusCode::UNAUTHORIZED);
    let payload: ErrorResponse = read_json(me_response).await?;
    assert_eq!(payload.code, "auth_session_expired");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn cannot_remove_last_platform_admin() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin users last admin").await? else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_id: i64 = sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin'")
        .fetch_one(&pool)
        .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{admin_id}"))
                .header(header::COOKIE, admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"is_platform_admin":false}"#))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "last_platform_admin_required");

    teardown(&database_url, &schema, pool).await
}
