use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuthSessionResponse {
    pub user: AuthUser,
    pub auth_mode: String,
}

#[cfg(test)]
mod tests {
    use super::{AuthSessionResponse, AuthUser};

    #[test]
    fn auth_session_response_serializes_expected_shape() {
        let value = serde_json::to_value(AuthSessionResponse {
            user: AuthUser {
                id: 1,
                username: "admin".to_owned(),
            },
            auth_mode: "session".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "user": {
                    "id": 1,
                    "username": "admin"
                },
                "auth_mode": "session"
            })
        );
    }
}
