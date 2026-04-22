use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlatformProjectSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub status: String,
    pub member_count: i64,
    pub deployment_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlatformProjectListResponse {
    pub items: Vec<PlatformProjectSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreatePlatformProjectRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub initial_admin_user_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlatformProject {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlatformProjectInitialAdmin {
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CreatePlatformProjectResponse {
    pub project: PlatformProject,
    pub initial_admin: PlatformProjectInitialAdmin,
}

#[cfg(test)]
mod tests {
    use super::{
        CreatePlatformProjectResponse, PlatformProject, PlatformProjectInitialAdmin,
        PlatformProjectListResponse, PlatformProjectSummary,
    };

    #[test]
    fn platform_project_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(PlatformProjectListResponse {
            items: vec![PlatformProjectSummary {
                id: 3,
                code: "coffee-main".to_owned(),
                name: "Coffee Main".to_owned(),
                status: "active".to_owned(),
                member_count: 4,
                deployment_count: 18,
                created_at: "2026-04-22T11:30:00Z".to_owned(),
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
                        "id": 3,
                        "code": "coffee-main",
                        "name": "Coffee Main",
                        "status": "active",
                        "member_count": 4,
                        "deployment_count": 18,
                        "created_at": "2026-04-22T11:30:00Z"
                    }
                ],
                "total": 1,
                "page": 1,
                "page_size": 20
            })
        );
    }

    #[test]
    fn create_platform_project_response_serializes_expected_shape() {
        let value = serde_json::to_value(CreatePlatformProjectResponse {
            project: PlatformProject {
                id: 3,
                code: "coffee-main".to_owned(),
                name: "Coffee Main".to_owned(),
                description: Some("Coffee config center".to_owned()),
                status: "active".to_owned(),
            },
            initial_admin: PlatformProjectInitialAdmin {
                user_id: 12,
                username: "alice".to_owned(),
                role: "admin".to_owned(),
            },
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "project": {
                    "id": 3,
                    "code": "coffee-main",
                    "name": "Coffee Main",
                    "description": "Coffee config center",
                    "status": "active"
                },
                "initial_admin": {
                    "user_id": 12,
                    "username": "alice",
                    "role": "admin"
                }
            })
        );
    }
}
