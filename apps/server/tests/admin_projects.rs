#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{admin_project::CreatePlatformProjectResponse, project::ProjectListResponse};
use server::error::ErrorResponse;
use support::{TestResult, login_as, read_json, seed_user, setup_app, teardown};
use tower::util::ServiceExt;

#[tokio::test]
async fn platform_admin_creates_project_without_implicit_membership_and_initial_admin_can_see_it()
-> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin projects create").await? else {
        return Ok(());
    };

    let initial_admin_user_id = seed_user(&pool, "alice", "alice1234").await?;
    let platform_admin_cookie = login_as(&app, "admin", "admin123456").await?;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/projects")
                .header(header::COOKIE, &platform_admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"coffee-main","name":"Coffee Main","description":"Coffee config center","initial_admin_user_id":{initial_admin_user_id}}}"#,
                )))?,
        )
        .await?;

    assert_eq!(create_response.status(), StatusCode::CREATED);
    let payload: CreatePlatformProjectResponse = read_json(create_response).await?;
    assert_eq!(payload.project.code, "coffee-main");
    assert_eq!(payload.initial_admin.user_id, initial_admin_user_id);
    assert_eq!(payload.initial_admin.username, "alice");
    assert_eq!(payload.initial_admin.role, "admin");

    let platform_admin_projects_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, &platform_admin_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(platform_admin_projects_response.status(), StatusCode::OK);
    let platform_admin_projects: ProjectListResponse =
        read_json(platform_admin_projects_response).await?;
    assert!(platform_admin_projects.items.is_empty());

    let initial_admin_cookie = login_as(&app, "alice", "alice1234").await?;
    let initial_admin_projects_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects")
                .header(header::COOKIE, &initial_admin_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(initial_admin_projects_response.status(), StatusCode::OK);
    let initial_admin_projects: ProjectListResponse =
        read_json(initial_admin_projects_response).await?;
    assert_eq!(initial_admin_projects.items.len(), 1);
    assert_eq!(initial_admin_projects.items[0].code, "coffee-main");
    assert_eq!(initial_admin_projects.items[0].current_user_role, "admin");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn platform_admin_deletes_empty_project_without_project_membership() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("admin projects delete").await? else {
        return Ok(());
    };

    let initial_admin_user_id = seed_user(&pool, "alice", "alice1234").await?;
    let platform_admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/projects")
                .header(header::COOKIE, &platform_admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"empty-admin-project","name":"Empty Admin Project","initial_admin_user_id":{initial_admin_user_id}}}"#,
                )))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: CreatePlatformProjectResponse = read_json(create_response).await?;

    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/projects/{}", created.project.id))
                .header(header::COOKIE, platform_admin_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = $1")
        .bind(created.project.id)
        .fetch_one(&pool)
        .await?;
    assert_eq!(project_count, 0);

    let deleted_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs WHERE action = 'project.deleted' AND resource_id = $1 AND project_id IS NULL",
    )
    .bind(created.project.id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(deleted_audit_count, 1);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn platform_project_delete_rejects_dependent_resources() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app("admin projects delete conflict").await?
    else {
        return Ok(());
    };

    let initial_admin_user_id = seed_user(&pool, "alice", "alice1234").await?;
    let platform_admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/projects")
                .header(header::COOKIE, &platform_admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"code":"used-admin-project","name":"Used Admin Project","initial_admin_user_id":{initial_admin_user_id}}}"#,
                )))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: CreatePlatformProjectResponse = read_json(create_response).await?;
    sqlx::query(
        "INSERT INTO config_files (project_id, code, name, format, sensitivity, status) VALUES ($1, 'main', 'Main', 'yaml', 'normal', 'active')",
    )
    .bind(created.project.id)
    .execute(&pool)
    .await?;

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admin/projects/{}", created.project.id))
                .header(header::COOKIE, platform_admin_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(delete_response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(delete_response).await?;
    assert_eq!(payload.code, "project_delete_conflict");

    teardown(&database_url, &schema, pool).await
}
