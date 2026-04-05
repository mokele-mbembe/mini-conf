pub mod bootstrap;
pub mod config;
pub mod error;
mod http;
pub mod state;

use axum::Router;
use state::AppState;

pub fn app(state: AppState) -> Router {
    http::router(state)
}

#[cfg(test)]
mod tests {
    use crate::{config::AppConfig, error::ErrorResponse, state::AppState};
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode, header},
    };
    use infra::AppIdentity;
    use schema::health::HealthzResponse;
    use tower::util::ServiceExt;

    fn test_app() -> axum::Router {
        super::app(AppState::new(
            AppIdentity::new("mini-conf-server", "test-version"),
            AppConfig::default(),
            None,
        ))
    }

    #[tokio::test]
    async fn healthz_returns_ok_payload() {
        let app = test_app();

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

    #[tokio::test]
    async fn unknown_route_returns_json_not_found() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/missing")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&header::HeaderValue::from_static("application/json"))
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: ErrorResponse =
            serde_json::from_slice(&body).expect("payload should be valid json");

        assert_eq!(
            payload,
            ErrorResponse {
                code: "route_not_found".to_owned(),
                message: "Route not found".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn healthz_without_api_prefix_returns_json_not_found() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let payload: ErrorResponse =
            serde_json::from_slice(&body).expect("payload should be valid json");

        assert_eq!(
            payload,
            ErrorResponse {
                code: "route_not_found".to_owned(),
                message: "Route not found".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn healthz_rejects_non_get_methods() {
        let app = test_app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/healthz")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
