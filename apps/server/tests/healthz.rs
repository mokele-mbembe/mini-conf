use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use infra::AppIdentity;
use schema::health::HealthzResponse;
use tower::util::ServiceExt;

#[tokio::test]
async fn healthz_endpoint_returns_json_payload() {
    let app = server::app(AppIdentity::new("mini-conf-server", "integration-test"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/healthz")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/json"))
    );

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    let payload: HealthzResponse =
        serde_json::from_slice(&body).expect("payload should be valid json");

    assert_eq!(
        payload,
        HealthzResponse {
            status: "ok".to_owned(),
            service: "mini-conf-server".to_owned(),
            version: "integration-test".to_owned(),
        }
    );
}
