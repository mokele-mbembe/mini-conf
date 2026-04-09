#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{project::ProjectSummary, project_member::ProjectMemberListResponse};
use server::error::ErrorResponse;
use support::{TestResult, login_as, read_json, seed_user, setup_app, teardown};
use tower::util::ServiceExt;

#[tokio::test]
async fn create_project_assigns_creator_as_admin_member() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project members create").await? else {
        return Ok(());
    };
    seed_user(&pool, "alice", "alice123").await?;

    let cookie = login_as(&app, "alice", "alice123").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"alice-project","name":"Alice Project"}"#,
                ))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);
    let project: ProjectSummary = read_json(response).await?;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}/members", project.id))
                .header(header::COOKIE, &cookie)
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
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects")
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"member-guard","name":"Member Guard"}"#,
                ))?,
        )
        .await?;
    let project: ProjectSummary = read_json(response).await?;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}/members", project.id))
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
                    project.id
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
                .uri(format!("/api/projects/{}/members", project.id))
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
                    project.id
                ))
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    teardown(&database_url, &schema, pool).await
}
