use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::AppIdentity;
use schema::health::HealthzResponse;
use server::config::AppConfig;
use server::error::ErrorResponse;
use server::state::AppState;
use tower::util::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn test_app() -> axum::Router {
    server::app(AppState::new(
        AppIdentity::new("mini-conf-server", "integration-test"),
        AppConfig::default(),
        None,
    ))
}

async fn read_json<T: serde::de::DeserializeOwned>(
    response: axum::response::Response,
) -> TestResult<T> {
    let body = to_bytes(response.into_body(), usize::MAX).await?;

    Ok(serde_json::from_slice(&body)?)
}

#[tokio::test]
async fn healthz_endpoint_returns_json_payload() -> TestResult {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/api/healthz").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/json"))
    );

    let payload: HealthzResponse = read_json(response).await?;

    assert_eq!(
        payload,
        HealthzResponse {
            status: "ok".to_owned(),
            service: "mini-conf-server".to_owned(),
            version: "integration-test".to_owned(),
        }
    );
    Ok(())
}

#[tokio::test]
async fn unknown_api_route_returns_json_not_found_payload() -> TestResult {
    let app = test_app();

    let response = app
        .oneshot(Request::builder().uri("/api/unknown").body(Body::empty())?)
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/json"))
    );

    let payload: ErrorResponse = read_json(response).await?;

    assert_eq!(
        payload,
        ErrorResponse {
            code: "route_not_found".to_owned(),
            message: "Route not found".to_owned(),
        }
    );
    Ok(())
}

#[tokio::test]
async fn head_healthz_returns_success_without_response_body() -> TestResult {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri("/api/healthz")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/json"))
    );

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    assert!(body.is_empty(), "HEAD responses should not include a body");
    Ok(())
}

#[tokio::test]
async fn healthz_accepts_explicit_json_accept_header() -> TestResult {
    let app = test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let payload: HealthzResponse = read_json(response).await?;
    assert_eq!(payload.status, "ok");
    Ok(())
}
