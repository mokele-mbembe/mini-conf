use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProjectSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ProjectListResponse {
    pub items: Vec<ProjectSummary>,
}

#[cfg(test)]
mod tests {
    use super::{ProjectListResponse, ProjectSummary};

    #[test]
    fn project_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(ProjectListResponse {
            items: vec![ProjectSummary {
                id: 1,
                code: "coffee-legacy".to_owned(),
                name: "Coffee Legacy".to_owned(),
                description: Some("Retail edge project".to_owned()),
                status: "active".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "code": "coffee-legacy",
                        "name": "Coffee Legacy",
                        "description": "Retail edge project",
                        "status": "active"
                    }
                ]
            })
        );
    }
}
