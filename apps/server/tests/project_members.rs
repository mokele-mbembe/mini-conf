#[path = "support/mod.rs"]
mod support;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{
    project::ProjectSummary,
    project_member::{ProjectMemberListResponse, ProjectMemberSummary},
};
use server::error::ErrorResponse;
use sqlx::PgPool;
use support::{
    TestResult, create_platform_project, login_as, lookup_user_id, read_json, seed_user, setup_app,
    teardown,
};
use tower::util::ServiceExt;

async fn create_project(
    app: &Router,
    pool: &PgPool,
    cookie: &str,
    code: &str,
) -> TestResult<ProjectSummary> {
    let admin_user_id = lookup_user_id(pool, "admin").await?;
    let created = create_platform_project(
        app,
        cookie,
        code,
        &format!("{code} Project"),
        None,
        admin_user_id,
    )
    .await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}", created.project.id))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    read_json(response).await
}

async fn list_project_members(
    app: &Router,
    cookie: &str,
    project_id: i64,
) -> TestResult<ProjectMemberListResponse> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}/members"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    read_json(response).await
}

async fn add_project_member(
    app: &Router,
    cookie: &str,
    project_id: i64,
    username: &str,
    role: &str,
) -> TestResult<ProjectMemberSummary> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/members"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"username":{},"role":{}}}"#,
                    serde_json::to_string(username)?,
                    serde_json::to_string(role)?,
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    read_json(response).await
}

async fn update_project_member(
    app: &Router,
    cookie: &str,
    project_id: i64,
    member_id: i64,
    role: &str,
) -> TestResult<ProjectMemberSummary> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{project_id}/members/{member_id}"))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"role":{}}}"#,
                    serde_json::to_string(role)?,
                )))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    read_json(response).await
}

#[tokio::test]
async fn create_project_uses_explicit_initial_admin_membership() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members create").await? else {
        return Ok(());
    };
    seed_user(&pool, "alice", "alice123").await?;

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let alice_user_id = lookup_user_id(&pool, "alice").await?;
    let created = create_platform_project(
        &app,
        &admin_cookie,
        "alice-project",
        "Alice Project",
        None,
        alice_user_id,
    )
    .await?;
    let project_id = created.project.id;

    let alice_cookie = login_as(&app, "alice", "alice123").await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}/members"))
                .header(header::COOKIE, &alice_cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let payload: ProjectMemberListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].username, "alice");
    assert_eq!(payload.items[0].role, "admin");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_member_endpoints_enforce_last_admin_guard() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members guard").await? else {
        return Ok(());
    };
    seed_user(&pool, "bob", "bob123").await?;

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let created = create_platform_project(
        &app,
        &admin_cookie,
        "member-guard",
        "Member Guard",
        None,
        admin_user_id,
    )
    .await?;
    let project_id = created.project.id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}/members"))
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    let members: ProjectMemberListResponse = read_json(response).await?;
    let admin_member_id = members
        .items
        .iter()
        .find(|item| item.username == "admin")
        .ok_or_else(|| std::io::Error::other("admin member should exist"))?
        .id;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/projects/{}/members/{admin_member_id}",
                    project_id
                ))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"role":"viewer"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "last_project_admin_required");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/members"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"bob","role":"admin"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/projects/{}/members/{admin_member_id}",
                    project_id
                ))
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_member_crud_flow_allows_admin_to_manage_members() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members crud").await? else {
        return Ok(());
    };
    seed_user(&pool, "carol", "carol123").await?;

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let project = create_project(&app, &pool, &admin_cookie, "member-crud").await?;

    let created = add_project_member(&app, &admin_cookie, project.id, "carol", "viewer").await?;
    assert_eq!(created.username, "carol");
    assert_eq!(created.role, "viewer");

    let updated =
        update_project_member(&app, &admin_cookie, project.id, created.id, "editor").await?;
    assert_eq!(updated.username, "carol");
    assert_eq!(updated.role, "editor");

    let members = list_project_members(&app, &admin_cookie, project.id).await?;
    assert!(members.items.iter().any(|item| {
        item.id == created.id && item.username == "carol" && item.role == "editor"
    }));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/projects/{}/members/{}",
                    project.id, created.id
                ))
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let members = list_project_members(&app, &admin_cookie, project.id).await?;
    assert!(!members.items.iter().any(|item| item.id == created.id));

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn non_admin_project_members_cannot_manage_members() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members permissions").await?
    else {
        return Ok(());
    };
    seed_user(&pool, "dave", "dave123").await?;
    seed_user(&pool, "erin", "erin123").await?;

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let project = create_project(&app, &pool, &admin_cookie, "member-permissions").await?;
    let dave = add_project_member(&app, &admin_cookie, project.id, "dave", "viewer").await?;

    let viewer_cookie = login_as(&app, "dave", "dave123").await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{}/members", project.id))
                .header(header::COOKIE, &viewer_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"erin","role":"viewer"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "project_permission_denied");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/projects/{}/members/{}", project.id, dave.id))
                .header(header::COOKIE, &viewer_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"role":"editor"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "project_permission_denied");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/projects/{}/members/{}", project.id, dave.id))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "project_permission_denied");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn create_project_member_returns_not_found_for_unknown_user() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members unknown user").await?
    else {
        return Ok(());
    };

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let project = create_project(&app, &pool, &admin_cookie, "member-unknown-user").await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{}/members", project.id))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username":"missing","role":"viewer"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "user_not_found");

    teardown(&database_url, &schema, pool).await
}
