#[path = "support/mod.rs"]
mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use schema::project_environment::{ProjectEnvironmentListResponse, ProjectEnvironmentSummary};
use server::error::ErrorResponse;
use sqlx::PgPool;
use support::{
    TestResult, grant_project_role, login_as, read_json, seed_deployment_instance,
    seed_project_environment, seed_user, setup_app, teardown,
};
use tower::util::ServiceExt;

async fn seed_project(pool: &PgPool, code: &str, name: &str) -> TestResult<i64> {
    let project_id: i64 = sqlx::query_scalar(
        "INSERT INTO projects (code, name, status) VALUES ($1, $2, 'active') RETURNING id",
    )
    .bind(code)
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(project_id)
}

#[tokio::test]
async fn project_environment_crud_flow_works_for_project_admin() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project environments crud").await?
    else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-legacy", "Coffee Legacy").await?;
    let _member_id = grant_project_role(&pool, project_id, "admin", "admin").await?;
    let cookie = login_as(&app, "admin", "admin123456").await?;

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"prod","name":"Production","description":"primary env","status":"active","sort_order":10}"#,
                ))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::CREATED);
    let created: ProjectEnvironmentSummary = read_json(create_response).await?;
    assert_eq!(created.project_id, project_id);
    assert_eq!(created.code, "prod");
    assert_eq!(created.name, "Production");
    assert_eq!(created.description.as_deref(), Some("primary env"));
    assert_eq!(created.status, "active");
    assert_eq!(created.sort_order, 10);
    assert_eq!(created.deployment_count, 0);

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed: ProjectEnvironmentListResponse = read_json(list_response).await?;
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id, created.id);

    let detail_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/projects/{project_id}/environments/{}",
                    created.id
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(detail_response.status(), StatusCode::OK);
    let detail: ProjectEnvironmentSummary = read_json(detail_response).await?;
    assert_eq!(detail.id, created.id);
    assert_eq!(detail.code, "prod");

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/projects/{project_id}/environments/{}",
                    created.id
                ))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"name":"Production Main","description":"main prod env","status":"inactive","sort_order":20}"#,
                ))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: ProjectEnvironmentSummary = read_json(update_response).await?;
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.code, "prod");
    assert_eq!(updated.name, "Production Main");
    assert_eq!(updated.description.as_deref(), Some("main prod env"));
    assert_eq!(updated.status, "inactive");
    assert_eq!(updated.sort_order, 20);

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/projects/{project_id}/environments/{}",
                    created.id
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_environment_endpoints_enforce_permissions() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app("project environments permissions").await?
    else {
        return Ok(());
    };

    seed_user(&pool, "alice", "alice123").await?;
    let project_id = seed_project(&pool, "ops-console", "Ops Console").await?;
    let _admin_member_id = grant_project_role(&pool, project_id, "admin", "admin").await?;
    let _viewer_member_id = grant_project_role(&pool, project_id, "alice", "viewer").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active").await?;

    let viewer_cookie = login_as(&app, "alice", "alice123").await?;
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(list_response.status(), StatusCode::OK);

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &viewer_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"staging","name":"Staging","status":"active"}"#,
                ))?,
        )
        .await?;
    assert_eq!(create_response.status(), StatusCode::FORBIDDEN);
    let create_error: ErrorResponse = read_json(create_response).await?;
    assert_eq!(create_error.code, "project_permission_denied");

    let update_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!(
                    "/api/projects/{project_id}/environments/{environment_id}"
                ))
                .header(header::COOKIE, &viewer_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"name":"Production","status":"active","sort_order":10}"#,
                ))?,
        )
        .await?;
    assert_eq!(update_response.status(), StatusCode::FORBIDDEN);

    let delete_response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/projects/{project_id}/environments/{environment_id}"
                ))
                .header(header::COOKIE, &viewer_cookie)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(delete_response.status(), StatusCode::FORBIDDEN);

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_environment_create_rejects_duplicate_code_and_invalid_status() -> TestResult {
    let Some((app, pool, database_url, schema)) =
        setup_app("project environments validation").await?
    else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-admin", "Coffee Admin").await?;
    let _member_id = grant_project_role(&pool, project_id, "admin", "admin").await?;
    let _environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active").await?;
    let cookie = login_as(&app, "admin", "admin123456").await?;

    let duplicate_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"prod","name":"Duplicate Prod","status":"active"}"#,
                ))?,
        )
        .await?;
    assert_eq!(duplicate_response.status(), StatusCode::CONFLICT);
    let duplicate_error: ErrorResponse = read_json(duplicate_response).await?;
    assert_eq!(duplicate_error.code, "project_environment_code_conflict");

    let invalid_status_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/projects/{project_id}/environments"))
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"code":"staging","name":"Staging","status":"paused"}"#,
                ))?,
        )
        .await?;
    assert_eq!(invalid_status_response.status(), StatusCode::BAD_REQUEST);
    let invalid_status_error: ErrorResponse = read_json(invalid_status_response).await?;
    assert_eq!(invalid_status_error.code, "invalid_request");
    assert_eq!(
        invalid_status_error.message,
        "invalid project environment status"
    );

    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn project_environment_delete_rejects_environment_in_use() -> TestResult {
    let Some((app, pool, database_url, schema)) = setup_app("project environments in use").await?
    else {
        return Ok(());
    };

    let project_id = seed_project(&pool, "coffee-prod", "Coffee Prod").await?;
    let _member_id = grant_project_role(&pool, project_id, "admin", "admin").await?;
    let environment_id =
        seed_project_environment(&pool, project_id, "prod", "Production", "active").await?;
    let _deployment_id = seed_deployment_instance(&pool, project_id, "store-001", false).await?;
    let cookie = login_as(&app, "admin", "admin123456").await?;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!(
                    "/api/projects/{project_id}/environments/{environment_id}"
                ))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let payload: ErrorResponse = read_json(response).await?;
    assert_eq!(payload.code, "environment_in_use");

    teardown(&database_url, &schema, pool).await
}
