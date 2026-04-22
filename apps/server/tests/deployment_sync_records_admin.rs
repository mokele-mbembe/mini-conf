#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::audit::DeploymentSyncRecordListResponse;
use support::{
    TestResult, create_platform_project, grant_project_role, login_as, lookup_user_id, read_json,
    seed_config_file, seed_deployment_instance, seed_release, seed_sync_record, seed_user,
    setup_app, teardown,
};
use tower::util::ServiceExt;

#[tokio::test]
async fn deployment_sync_records_list_is_scoped_to_visible_projects() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app("deployment sync records admin").await?
    else {
        return Ok(());
    };
    seed_user(&pool, "viewer-sync", "viewer123").await?;
    seed_user(&pool, "outsider-sync", "outsider123").await?;

    let admin_cookie = login_as(&app, "admin", "admin123456").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let project = create_platform_project(
        &app,
        &admin_cookie,
        "sync-project",
        "Sync Project",
        None,
        admin_user_id,
    )
    .await?;
    grant_project_role(&pool, project.project.id, "viewer-sync", "viewer").await?;

    let config_file_id = seed_config_file(&pool, project.project.id, "main").await?;
    let deployment_id =
        seed_deployment_instance(&pool, project.project.id, "store-001", false).await?;
    let release_id = seed_release(
        &pool,
        project.project.id,
        config_file_id,
        deployment_id,
        "20260410.0001",
    )
    .await?;
    seed_sync_record(
        &pool,
        project.project.id,
        deployment_id,
        config_file_id,
        release_id,
    )
    .await?;

    let viewer_cookie = login_as(&app, "viewer-sync", "viewer123").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-sync-records?project_id={}&config_file_id={}",
                    project.project.id, config_file_id
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentSyncRecordListResponse = read_json(response).await?;
    assert_eq!(payload.items.len(), 1);
    assert_eq!(payload.items[0].config_file_id, config_file_id);
    assert_eq!(payload.items[0].config, "main");

    let outsider_cookie = login_as(&app, "outsider-sync", "outsider123").await?;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-sync-records?project_id={}",
                    project.project.id
                ))
                .header(header::COOKIE, &outsider_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload: DeploymentSyncRecordListResponse = read_json(response).await?;
    assert!(payload.items.is_empty());

    teardown(&database_url, &schema, pool).await
}
