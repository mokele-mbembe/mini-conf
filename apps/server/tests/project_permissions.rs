#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{
    audit::DeploymentSyncRecordListResponse, draft::DraftResponse, project::ProjectSummary,
    release::ReleaseListResponse,
};
use server::error::ErrorResponse;
use sqlx::PgPool;
use support::{
    TestResult, grant_project_role, login_as, read_json, seed_config_file,
    seed_deployment_instance, seed_release, seed_sync_record, seed_user, setup_app, teardown,
};
use tower::util::ServiceExt;

async fn seed_draft(
    pool: &PgPool,
    project_id: i64,
    deployment_id: i64,
    config_file_id: i64,
) -> TestResult {
    let admin_user_id: i64 =
        sqlx::query_scalar("SELECT id FROM users WHERE username = 'admin' LIMIT 1")
            .fetch_one(pool)
            .await?;
    sqlx::query(
        r#"
        INSERT INTO drafts (
            project_id,
            config_file_id,
            deployment_instance_id,
            content,
            content_hash,
            format,
            schema_version,
            version,
            editor_user_id
        )
        VALUES ($1, $2, $3, 'log_level: info\n', 'abc123', 'yaml', 'v1', 1, $4)
        "#,
    )
    .bind(project_id)
    .bind(config_file_id)
    .bind(deployment_id)
    .bind(admin_user_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn viewer_can_read_release_and_sync_records_but_not_draft_or_preview() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project permissions viewer").await?
    else {
        return Ok(());
    };
    seed_user(&pool, "viewer1", "viewer123").await?;
    seed_user(&pool, "outsider1", "outsider123").await?;

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
                    r#"{"code":"viewer-project","name":"Viewer Project"}"#,
                ))?,
        )
        .await?;
    let project: ProjectSummary = read_json(response).await?;
    grant_project_role(&pool, project.id, "viewer1", "viewer").await?;

    let config_file_id = seed_config_file(&pool, project.id, "main").await?;
    let deployment_id = seed_deployment_instance(&pool, project.id, "store-001", false).await?;
    seed_draft(&pool, project.id, deployment_id, config_file_id).await?;
    let release_id = seed_release(
        &pool,
        project.id,
        config_file_id,
        deployment_id,
        "20260410.0001",
    )
    .await?;
    seed_sync_record(&pool, project.id, deployment_id, config_file_id, release_id).await?;

    let viewer_cookie = login_as(&app, "viewer1", "viewer123").await?;
    let release_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/releases?project_id={}", project.id))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(release_response.status(), StatusCode::OK);
    let releases: ReleaseListResponse = read_json(release_response).await?;
    assert_eq!(releases.items.len(), 1);

    let draft_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(draft_response.status(), StatusCode::FORBIDDEN);
    let draft_error: ErrorResponse = read_json(draft_response).await?;
    assert_eq!(draft_error.code, "project_permission_denied");

    let preview_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/preview-bundle"
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(preview_response.status(), StatusCode::FORBIDDEN);

    let sync_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/deployment-sync-records?project_id={}",
                    project.id
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(sync_response.status(), StatusCode::OK);
    let sync_records: DeploymentSyncRecordListResponse = read_json(sync_response).await?;
    assert_eq!(sync_records.items.len(), 1);

    let outsider_cookie = login_as(&app, "outsider1", "outsider123").await?;
    let outsider_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{}", project.id))
                .header(header::COOKIE, &outsider_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(outsider_response.status(), StatusCode::NOT_FOUND);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn editor_can_save_draft_but_cannot_reset_token() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project permissions editor").await?
    else {
        return Ok(());
    };
    seed_user(&pool, "editor1", "editor123").await?;

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
                    r#"{"code":"editor-project","name":"Editor Project"}"#,
                ))?,
        )
        .await?;
    let project: ProjectSummary = read_json(response).await?;
    grant_project_role(&pool, project.id, "editor1", "editor").await?;

    let config_file_id = seed_config_file(&pool, project.id, "main").await?;
    let deployment_id = seed_deployment_instance(&pool, project.id, "store-001", false).await?;

    let editor_cookie = login_as(&app, "editor1", "editor123").await?;
    let draft_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &editor_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"log_level: debug\n","format":"yaml","base_version":0}"#,
                ))?,
        )
        .await?;
    assert_eq!(draft_response.status(), StatusCode::OK);
    let draft: DraftResponse = read_json(draft_response).await?;
    assert_eq!(draft.version, 1);

    let token_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/deployment-instances/{deployment_id}/token/reset"
                ))
                .header(header::COOKIE, &editor_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(token_response.status(), StatusCode::FORBIDDEN);
    let token_error: ErrorResponse = read_json(token_response).await?;
    assert_eq!(token_error.code, "project_permission_denied");

    teardown(&database_url, &schema, pool).await
}
