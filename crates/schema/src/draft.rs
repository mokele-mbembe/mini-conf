use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DraftResponse {
    pub deployment_instance_id: i64,
    pub config_file_id: i64,
    pub format: String,
    pub content: String,
    pub version: i64,
    pub schema_version: Option<String>,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::DraftResponse;

    #[test]
    fn draft_response_serializes_expected_shape() {
        let value = serde_json::to_value(DraftResponse {
            deployment_instance_id: 8,
            config_file_id: 3,
            format: "yaml".to_owned(),
            content: "poll_interval_ms: 5000".to_owned(),
            version: 4,
            schema_version: Some("v1".to_owned()),
            updated_at: "2026-04-05T12:00:00Z".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "deployment_instance_id": 8,
                "config_file_id": 3,
                "format": "yaml",
                "content": "poll_interval_ms: 5000",
                "version": 4,
                "schema_version": "v1",
                "updated_at": "2026-04-05T12:00:00Z"
            })
        );
    }
}
