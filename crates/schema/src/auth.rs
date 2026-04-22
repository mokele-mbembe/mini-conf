use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub is_platform_admin: bool,
    pub status: String,
    pub must_change_password: bool,
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
                is_platform_admin: true,
                status: "active".to_owned(),
                must_change_password: false,
            },
            auth_mode: "session".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "user": {
                    "id": 1,
                    "username": "admin",
                    "is_platform_admin": true,
                    "status": "active",
                    "must_change_password": false
                },
                "auth_mode": "session"
            })
        );
    }
}
