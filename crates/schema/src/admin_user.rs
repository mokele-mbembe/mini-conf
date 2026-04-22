use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminUserSummary {
    pub id: i64,
    pub username: String,
    pub status: String,
    pub is_platform_admin: bool,
    pub must_change_password: bool,
    pub last_login_at: Option<String>,
    pub password_updated_at: Option<String>,
    pub project_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminUserListResponse {
    pub items: Vec<AdminUserSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminUserProjectSummary {
    pub project_id: i64,
    pub project_code: String,
    pub project_name: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AdminUserDetail {
    pub id: i64,
    pub username: String,
    pub status: String,
    pub is_platform_admin: bool,
    pub must_change_password: bool,
    pub last_login_at: Option<String>,
    pub password_updated_at: Option<String>,
    pub projects: Vec<AdminUserProjectSummary>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreateAdminUserRequest {
    pub username: String,
    pub password: String,
    pub is_platform_admin: bool,
    pub must_change_password: bool,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UpdateAdminUserRequest {
    pub status: Option<String>,
    pub is_platform_admin: Option<bool>,
    pub must_change_password: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResetAdminUserPasswordRequest {
    pub new_password: String,
    pub must_change_password: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        AdminUserDetail, AdminUserListResponse, AdminUserProjectSummary, AdminUserSummary,
    };

    #[test]
    fn admin_user_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(AdminUserListResponse {
            items: vec![AdminUserSummary {
                id: 1,
                username: "alice".to_owned(),
                status: "active".to_owned(),
                is_platform_admin: false,
                must_change_password: true,
                last_login_at: Some("2026-04-22T10:00:00Z".to_owned()),
                password_updated_at: Some("2026-04-20T09:00:00Z".to_owned()),
                project_count: 3,
                created_at: "2026-04-01T00:00:00Z".to_owned(),
            }],
            total: 1,
            page: 1,
            page_size: 20,
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "username": "alice",
                        "status": "active",
                        "is_platform_admin": false,
                        "must_change_password": true,
                        "last_login_at": "2026-04-22T10:00:00Z",
                        "password_updated_at": "2026-04-20T09:00:00Z",
                        "project_count": 3,
                        "created_at": "2026-04-01T00:00:00Z"
                    }
                ],
                "total": 1,
                "page": 1,
                "page_size": 20
            })
        );
    }

    #[test]
    fn admin_user_detail_serializes_expected_shape() {
        let value = serde_json::to_value(AdminUserDetail {
            id: 12,
            username: "alice".to_owned(),
            status: "active".to_owned(),
            is_platform_admin: false,
            must_change_password: true,
            last_login_at: None,
            password_updated_at: Some("2026-04-22T11:00:00Z".to_owned()),
            projects: vec![AdminUserProjectSummary {
                project_id: 3,
                project_code: "coffee-main".to_owned(),
                project_name: "Coffee Main".to_owned(),
                role: "admin".to_owned(),
            }],
            created_at: "2026-04-22T11:00:00Z".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "id": 12,
                "username": "alice",
                "status": "active",
                "is_platform_admin": false,
                "must_change_password": true,
                "last_login_at": null,
                "password_updated_at": "2026-04-22T11:00:00Z",
                "projects": [
                    {
                        "project_id": 3,
                        "project_code": "coffee-main",
                        "project_name": "Coffee Main",
                        "role": "admin"
                    }
                ],
                "created_at": "2026-04-22T11:00:00Z"
            })
        );
    }
}
