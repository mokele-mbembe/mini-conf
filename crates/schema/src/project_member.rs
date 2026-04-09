use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(example = project_member_summary_example)]
pub struct ProjectMemberSummary {
    pub id: i64,
    pub project_id: i64,
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(example = project_member_list_response_example)]
pub struct ProjectMemberListResponse {
    pub items: Vec<ProjectMemberSummary>,
}

fn project_member_summary_example() -> serde_json::Value {
    serde_json::json!({
        "id": 12,
        "project_id": 7,
        "user_id": 9,
        "username": "alice",
        "role": "editor",
        "created_at": "2026-04-10T12:00:00Z"
    })
}

fn project_member_list_response_example() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": 12,
                "project_id": 7,
                "user_id": 9,
                "username": "alice",
                "role": "editor",
                "created_at": "2026-04-10T12:00:00Z"
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::{ProjectMemberListResponse, ProjectMemberSummary};

    #[test]
    fn project_member_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(ProjectMemberListResponse {
            items: vec![ProjectMemberSummary {
                id: 1,
                project_id: 7,
                user_id: 9,
                username: "alice".to_owned(),
                role: "editor".to_owned(),
                created_at: "2026-04-10T12:00:00Z".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "project_id": 7,
                        "user_id": 9,
                        "username": "alice",
                        "role": "editor",
                        "created_at": "2026-04-10T12:00:00Z"
                    }
                ]
            })
        );
    }
}
