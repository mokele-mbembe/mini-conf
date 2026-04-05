pub mod bootstrap;
pub mod config;
mod http;

use axum::Router;
use infra::AppIdentity;

pub fn app(identity: AppIdentity) -> Router {
    http::router(identity)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use infra::AppIdentity;
    use schema::health::HealthzResponse;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn healthz_returns_ok_payload() {
        let app = super::app(AppIdentity::new("mini-conf-server", "test-version"));

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
                version: "test-version".to_owned(),
            }
        );
    }
}
