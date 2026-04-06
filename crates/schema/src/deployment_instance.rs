use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInstanceSummary {
    pub id: i64,
    pub project_id: i64,
    pub environment: String,
    pub deployment_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_template: bool,
    pub template_source_id: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInstanceListResponse {
    pub items: Vec<DeploymentInstanceSummary>,
}

#[cfg(test)]
mod tests {
    use super::{DeploymentInstanceListResponse, DeploymentInstanceSummary};

    #[test]
    fn deployment_instance_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentInstanceListResponse {
            items: vec![DeploymentInstanceSummary {
                id: 1,
                project_id: 7,
                environment: "prod".to_owned(),
                deployment_key: "store-001".to_owned(),
                name: "Store 001".to_owned(),
                description: Some("Hangzhou store 001".to_owned()),
                is_template: false,
                template_source_id: None,
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
                        "project_id": 7,
                        "environment": "prod",
                        "deployment_key": "store-001",
                        "name": "Store 001",
                        "description": "Hangzhou store 001",
                        "is_template": false,
                        "template_source_id": null,
                        "status": "active"
                    }
                ]
            })
        );
    }
}
