#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{audit::DeploymentHeartbeatListResponse, project::ProjectSummary};
use server::error::ErrorResponse;
use support::{
    TestResult, grant_project_role, login_as, read_json, seed_deployment_instance, seed_user,
    setup_app, teardown,
};
use tower::util::ServiceExt;

#[tokio::test]
async fn deployment_heartbeats_are_visible_to_project_members_only() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("deployment heartbeats").await? else {
        return Ok(());
    };
    seed_user(&pool, "viewer2", "viewer123").await?;
    seed_user(&pool, "outsider2", "outsider123").await?;

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
                    r#"{"code":"heartbeat-project","name":"Heartbeat Project"}"#,
                ))?,
        )
        .await?;
    let project: ProjectSummary = read_json(response).await?;
    grant_project_role(&pool, project.id, "viewer2", "viewer").await?;
    let deployment_id = seed_deployment_instance(&pool, project.id, "store-001", false).await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_heartbeats (
            project_id,
            deployment_instance_id,
            process_key,
            metadata,
            reported_at
        )
        VALUES ($1, $2, 'main', '{"ip":"10.0.0.10","version":"alpha"}', '2026-04-10T12:01:00Z')
        "#,
    )
    .bind(project.id)
    .bind(deployment_id)
    .execute(&pool)
    .await?;

    let viewer_cookie = login_as(&app, "viewer2", "viewer123").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-heartbeats?project_id={}&deployment_instance_id={}",
                    project.id, deployment_id
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentHeartbeatListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].process_key, "main");

    let outsider_cookie = login_as(&app, "outsider2", "outsider123").await?;
    let forbidden = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-heartbeats?project_id={}",
                    project.id
                ))
                .header(header::COOKIE, &outsider_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(forbidden.status(), StatusCode::NOT_FOUND);
    let error: ErrorResponse = read_json(forbidden).await?;
    assert_eq!(error.code, "project_not_found");

    teardown(&database_url, &schema, pool).await
}
