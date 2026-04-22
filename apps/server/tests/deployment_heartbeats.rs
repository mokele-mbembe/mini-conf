#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::audit::DeploymentHeartbeatListResponse;
use server::error::ErrorResponse;
use support::{
    TestResult, create_platform_project, grant_project_role, login_as, lookup_user_id, read_json,
    seed_config_file, seed_deployment_instance, seed_user, setup_app, teardown,
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
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let project = create_platform_project(
        &app,
        &admin_cookie,
        "heartbeat-project",
        "Heartbeat Project",
        None,
        admin_user_id,
    )
    .await?;
    grant_project_role(&pool, project.project.id, "viewer2", "viewer").await?;
    let config_file_id = seed_config_file(&pool, project.project.id, "main").await?;
    let deployment_id =
        seed_deployment_instance(&pool, project.project.id, "store-001", false).await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_heartbeats (
            project_id,
            deployment_instance_id,
            config_file_id,
            metadata,
            reported_at
        )
        VALUES ($1, $2, $3, '{"ip":"10.0.0.10","version":"alpha"}', '2026-04-10T12:01:00Z')
        "#,
    )
    .bind(project.project.id)
    .bind(deployment_id)
    .bind(config_file_id)
    .execute(&pool)
    .await?;

    let viewer_cookie = login_as(&app, "viewer2", "viewer123").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-heartbeats?project_id={}&deployment_instance_id={}&config_file_id={}",
                    project.project.id, deployment_id, config_file_id
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentHeartbeatListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].config_file_id, config_file_id);
    assert_eq!(payload.items[0].config, "main");

    let outsider_cookie = login_as(&app, "outsider2", "outsider123").await?;
    let forbidden = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-heartbeats?project_id={}",
                    project.project.id
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
