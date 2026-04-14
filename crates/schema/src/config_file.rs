use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConfigFileSummary {
    pub id: i64,
    pub project_id: i64,
    pub code: String,
    pub name: String,
    pub is_required: bool,
    pub format: String,
    pub sensitivity: String,
    pub secret_paths: Option<Vec<String>>,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConfigFileListResponse {
    pub items: Vec<ConfigFileSummary>,
}

#[cfg(test)]
mod tests {
    use super::{ConfigFileListResponse, ConfigFileSummary};

    #[test]
    fn config_file_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(ConfigFileListResponse {
            items: vec![ConfigFileSummary {
                id: 1,
                project_id: 3,
                code: "main".to_owned(),
                name: "Main Config".to_owned(),
                is_required: true,
                format: "yaml".to_owned(),
                sensitivity: "secret".to_owned(),
                secret_paths: Some(vec![
                    "$.wifi.password".to_owned(),
                    "$.third_party.api_key".to_owned(),
                ]),
                description: Some("Primary device configuration".to_owned()),
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
                        "project_id": 3,
                        "code": "main",
                        "name": "Main Config",
                        "is_required": true,
                        "format": "yaml",
                        "sensitivity": "secret",
                        "secret_paths": ["$.wifi.password", "$.third_party.api_key"],
                        "description": "Primary device configuration",
                        "status": "active"
                    }
                ]
            })
        );
    }
}
