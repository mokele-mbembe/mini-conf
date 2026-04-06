use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseSummary {
    pub id: i64,
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub config_file_id: i64,
    pub revision: String,
    pub content_hash: String,
    pub format: String,
    pub change_summary: Option<String>,
    pub apply_mode: String,
    pub published_by: i64,
    pub published_at: String,
}

#[cfg(test)]
mod tests {
    use super::ReleaseSummary;

    #[test]
    fn release_summary_serializes_expected_shape() {
        let value = serde_json::to_value(ReleaseSummary {
            id: 12,
            project_id: 1,
            deployment_instance_id: 8,
            config_file_id: 3,
            revision: "20260406.0001".to_owned(),
            content_hash: "abc123".to_owned(),
            format: "yaml".to_owned(),
            change_summary: Some("increase polling interval".to_owned()),
            apply_mode: "soft".to_owned(),
            published_by: 1,
            published_at: "2026-04-06T12:00:00Z".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "id": 12,
                "project_id": 1,
                "deployment_instance_id": 8,
                "config_file_id": 3,
                "revision": "20260406.0001",
                "content_hash": "abc123",
                "format": "yaml",
                "change_summary": "increase polling interval",
                "apply_mode": "soft",
                "published_by": 1,
                "published_at": "2026-04-06T12:00:00Z"
            })
        );
    }
}
