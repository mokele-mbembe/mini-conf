mod support;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use infra::AppIdentity;
use server::{
    config::{AppConfig, AppEnv},
    error::ErrorResponse,
    security::OPEN_API_RATE_LIMIT,
    state::AppState,
};
use sqlx::Row;
use support::TestResult;
use tower::util::ServiceExt;

const OPEN_RESOLVE_URL: &str = "/api/open/configs/resolve?project=coffee-legacy&environment=prod&deployment_key=store-001&config=main";

fn app_without_database(config: AppConfig) -> axum::Router {
    server::app(AppState::new(
        AppIdentity::new("mini-conf-server", "integration-test"),
        config,
        None,
    ))
}

#[tokio::test]
async fn open_api_missing_token_failure_is_audited() -> TestResult {
    let Some((app, pool, database_url, schema)) = support::setup_app("open api audit").await?
    else {
        return Ok(());
    };

    let response = app
        .oneshot(
            Request::builder()
                .uri(OPEN_RESOLVE_URL)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let row = sqlx::query(
        r#"
        SELECT action, resource_type, resource_id, detail
        FROM audit_logs
        WHERE action = 'open_api.request_failed'
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await?;

    let action: String = row.try_get("action")?;
    let resource_type: String = row.try_get("resource_type")?;
    let resource_id: String = row.try_get("resource_id")?;
    let detail: Option<serde_json::Value> = row.try_get("detail")?;
    let detail = detail.ok_or_else(|| std::io::Error::other("audit detail should exist"))?;

    assert_eq!(action, "open_api.request_failed");
    assert_eq!(resource_type, "open_api");
    assert_eq!(resource_id, "/api/open/configs/resolve");
    assert_eq!(detail["method"], "GET");
    assert_eq!(detail["path"], "/api/open/configs/resolve");
    assert_eq!(detail["status"], 401);
    assert_eq!(detail["has_bearer_token"], false);
    assert!(detail.get("token").is_none());

    support::teardown(&database_url, &schema, pool).await?;
    Ok(())
}

#[tokio::test]
async fn open_api_high_frequency_requests_are_rate_limited() -> TestResult {
    let app = app_without_database(AppConfig::default());

    for _ in 0..OPEN_API_RATE_LIMIT {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(OPEN_RESOLVE_URL)
                    .body(Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    let response = app
        .oneshot(
            Request::builder()
                .uri(OPEN_RESOLVE_URL)
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    let payload: ErrorResponse = support::read_json(response).await?;
    assert_eq!(
        payload,
        ErrorResponse {
            code: "open_api_rate_limited".to_owned(),
            message: "Too many Open API requests; try again later".to_owned(),
        }
    );

    Ok(())
}

#[tokio::test]
async fn production_responses_include_reviewed_security_headers() -> TestResult {
    let app = app_without_database(AppConfig {
        app_env: AppEnv::Prod,
        ..AppConfig::default()
    });

    let response = app
        .oneshot(Request::builder().uri("/api/healthz").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-content-type-options"),
        Some(&header::HeaderValue::from_static("nosniff"))
    );
    assert_eq!(
        response.headers().get("x-frame-options"),
        Some(&header::HeaderValue::from_static("DENY"))
    );
    assert_eq!(
        response.headers().get("cross-origin-resource-policy"),
        Some(&header::HeaderValue::from_static("same-origin"))
    );
    assert_eq!(
        response.headers().get("content-security-policy"),
        Some(&header::HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'"
        ))
    );
    assert_eq!(
        response.headers().get("strict-transport-security"),
        Some(&header::HeaderValue::from_static(
            "max-age=31536000; includeSubDomains"
        ))
    );

    Ok(())
}
