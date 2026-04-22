#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::audit::AuditLogListResponse;
use server::error::ErrorResponse;
use support::{
    TestResult, create_platform_project, grant_project_role, login_as, lookup_user_id, read_json,
    seed_config_file, seed_deployment_instance, seed_user, setup_app, teardown,
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
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let project = create_platform_project(
        &app,
        &admin_cookie,
        "audit-project",
        "Audit Project",
        None,
        admin_user_id,
    )
    .await?;
    grant_project_role(&pool, project.project.id, "viewer2", "viewer").await?;

    let config_file_id = seed_config_file(&pool, project.project.id, "main").await?;
    let deployment_id =
        seed_deployment_instance(&pool, project.project.id, "store-001", false).await?;
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
                .uri(format!("/api/audit-logs?project_id={}", project.project.id))
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
            .any(|item| item.action == "project.created_by_platform_admin")
    );

    let global_audit_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/audit-logs?action=project.created_by_platform_admin")
                .header(header::COOKIE, &admin_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(global_audit_response.status(), StatusCode::OK);
    let global_audits: AuditLogListResponse = read_json(global_audit_response).await?;
    assert!(global_audits.items.iter().any(|item| {
        item.action == "project.created_by_platform_admin"
            && item
                .detail
                .as_ref()
                .and_then(|detail| detail.get("project_code"))
                == Some(&serde_json::json!("audit-project"))
    }));
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
                .uri(format!("/api/audit-logs?project_id={}", project.project.id))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(forbidden_response.status(), StatusCode::FORBIDDEN);
    let error: ErrorResponse = read_json(forbidden_response).await?;
    assert_eq!(error.code, "project_permission_denied");

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn platform_admin_can_view_platform_level_audits_created_by_other_platform_admins()
-> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app("audit logs platform admin visibility").await?
    else {
        return Ok(());
    };

    let super_cookie = login_as(&app, "admin", "admin123456").await?;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/users")
                .header(header::COOKIE, &super_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"operator2","password":"Operator1234","is_platform_admin":true,"must_change_password":false,"status":"active"}"#,
                ))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);

    let operator_cookie = login_as(&app, "operator2", "Operator1234").await?;
    let admin_user_id = lookup_user_id(&pool, "admin").await?;
    let operator_user_id = lookup_user_id(&pool, "operator2").await?;
    let _ = create_platform_project(
        &app,
        &operator_cookie,
        "platform-visibility-project",
        "Platform Visibility Project",
        None,
        admin_user_id,
    )
    .await?;

    let root_audit_response = app
        .oneshot(
            Request::builder()
                .uri("/api/audit-logs?action=project.created_by_platform_admin")
                .header(header::COOKIE, &super_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(root_audit_response.status(), StatusCode::OK);
    let root_audits: AuditLogListResponse = read_json(root_audit_response).await?;
    assert!(root_audits.items.iter().any(|item| {
        item.user_id == Some(operator_user_id)
            && item.action == "project.created_by_platform_admin"
            && item
                .detail
                .as_ref()
                .and_then(|detail| detail.get("project_code"))
                == Some(&serde_json::json!("platform-visibility-project"))
    }));

    teardown(&database_url, &schema, pool).await
}
