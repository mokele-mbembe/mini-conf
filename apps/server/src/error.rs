use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ApiError {
    pub const fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "route_not_found",
            message: "Route not found",
        }
    }

    pub const fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_server_error",
            message: "Internal server error",
        }
    }

    pub fn into_body(self) -> ErrorResponse {
        ErrorResponse {
            code: self.code.to_owned(),
            message: self.message.to_owned(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status;

        (status, Json(self.into_body())).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{ApiError, ErrorResponse};
    use axum::{
        body::to_bytes,
        http::{StatusCode, header},
        response::IntoResponse,
    };

    #[tokio::test]
    async fn not_found_error_renders_json_response() {
        let response = ApiError::not_found().into_response();

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

    #[test]
    fn internal_error_builds_expected_body() {
        assert_eq!(
            ApiError::internal().into_body(),
            ErrorResponse {
                code: "internal_server_error".to_owned(),
                message: "Internal server error".to_owned(),
            }
        );
    }
}
