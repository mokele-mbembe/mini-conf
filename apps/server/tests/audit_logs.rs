#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::{audit::AuditLogListResponse, project::ProjectSummary};
use server::error::ErrorResponse;
use support::{
    TestResult, grant_project_role, login_as, read_json, seed_config_file,
    seed_deployment_instance, seed_user, setup_app, teardown,
};
use tower::util::ServiceExt;

#[tokio::test]
async fn audit_logs_include_project_events_without_sensitive_content() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("audit logs").await? else {
        return Ok(());
    };
    seed_user(&pool, "viewer2", "viewer123").await?;

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"wrong-password"}"#,
                ))?,
        )
        .await?;

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
                    r#"{"code":"audit-project","name":"Audit Project"}"#,
                ))?,
        )
        .await?;
    let project: ProjectSummary = read_json(response).await?;
    grant_project_role(&pool, project.id, "viewer2", "viewer").await?;

    let config_file_id = seed_config_file(&pool, project.id, "main").await?;
    let deployment_id = seed_deployment_instance(&pool, project.id, "store-001", false).await?;
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/drafts/{deployment_id}/{config_file_id}"))
                .header(header::COOKIE, &admin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"content":"log_level: info\n","format":"yaml","base_version":0}"#,
                ))?,
        )
        .await?;

    let audit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/audit-logs?project_id={}", project.id))
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(audit_response.status(), StatusCode::OK);
    let audits: AuditLogListResponse = read_json(audit_response).await?;
    assert!(
        audits
            .items
            .iter()
            .any(|item| item.action == "project.created")
    );
    let draft_entry = audits
        .items
        .iter()
        .find(|item| item.action == "draft.saved")
        .ok_or_else(|| std::io::Error::other("draft saved audit entry should exist"))?;
    assert!(
        draft_entry
            .detail
            .as_ref()
            .and_then(|detail| detail.get("content"))
            .is_none()
    );

    let viewer_cookie = login_as(&app, "viewer2", "viewer123").await?;
    let forbidden_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/audit-logs?project_id={}", project.id))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(forbidden_response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = read_json(forbidden_response).await?;
    assert_eq!(error.code, "project_permission_denied");

    teardown(&database_url, &schema, pool).await
}
