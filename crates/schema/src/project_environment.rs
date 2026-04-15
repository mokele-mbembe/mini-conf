use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProjectEnvironmentSummary {
    pub id: i64,
    pub project_id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub deployment_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProjectEnvironmentListResponse {
    pub items: Vec<ProjectEnvironmentSummary>,
}

#[cfg(test)]
mod tests {
    use super::{ProjectEnvironmentListResponse, ProjectEnvironmentSummary};

    #[test]
    fn project_environment_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(ProjectEnvironmentListResponse {
            items: vec![ProjectEnvironmentSummary {
                id: 1,
                project_id: 7,
                code: "prod".to_owned(),
                name: "Production".to_owned(),
                description: Some("primary traffic environment".to_owned()),
                status: "active".to_owned(),
                sort_order: 10,
                deployment_count: 3,
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
                        "code": "prod",
                        "name": "Production",
                        "description": "primary traffic environment",
                        "status": "active",
                        "sort_order": 10,
                        "deployment_count": 3
                    }
                ]
            })
        );
    }
}
