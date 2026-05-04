use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
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
    const fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
        }
    }

    pub const fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub const fn unauthorized(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub const fn forbidden(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub const fn unprocessable_entity(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, code, message)
    }

    pub const fn conflict(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub const fn too_many_requests(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, code, message)
    }

    pub const fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "route_not_found", "Route not found")
    }

    pub const fn not_found_with(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub const fn service_unavailable(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, code, message)
    }

    pub const fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_server_error",
            "Internal server error",
        )
    }

    #[track_caller]
    pub(crate) fn internal_with(error: impl Debug, context: &'static str) -> Self {
        let location = std::panic::Location::caller();
        tracing::error!(
            ?error,
            context,
            file = location.file(),
            line = location.line(),
            "internal server error"
        );
        Self::internal()
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

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

#[cfg(test)]
mod tests {
    use super::{ApiError, ErrorResponse};
    use axum::{
        body::to_bytes,
        http::{StatusCode, header},
        response::IntoResponse,
    };

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    #[tokio::test]
    async fn not_found_error_renders_json_response() -> TestResult {
        let response = ApiError::not_found().into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&header::HeaderValue::from_static("application/json"))
        );

        let body = to_bytes(response.into_body(), usize::MAX).await?;
        let payload: ErrorResponse = serde_json::from_slice(&body)?;

        assert_eq!(
            payload,
            ErrorResponse {
                code: "route_not_found".to_owned(),
                message: "Route not found".to_owned(),
            }
        );
        Ok(())
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

    #[test]
    fn bad_request_error_builds_expected_body() {
        assert_eq!(
            ApiError::bad_request("invalid_request", "missing required query parameter")
                .into_body(),
            ErrorResponse {
                code: "invalid_request".to_owned(),
                message: "missing required query parameter".to_owned(),
            }
        );
    }
}
