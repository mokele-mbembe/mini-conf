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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseListResponse {
    pub items: Vec<ReleaseSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDetailResponse {
    pub release: ReleaseSummary,
    pub content: String,
    pub diff_summary: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::{ReleaseDetailResponse, ReleaseListResponse, ReleaseSummary};

    #[test]
    fn release_summary_serializes_expected_shape() {
        let summary = ReleaseSummary {
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
        };
        let value = serde_json::to_value(&summary).expect("response should serialize");

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

    #[test]
    fn release_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(ReleaseListResponse {
            items: vec![ReleaseSummary {
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
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
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
                    }
                ]
            })
        );
    }

    #[test]
    fn release_detail_response_serializes_expected_shape() {
        let value = serde_json::to_value(ReleaseDetailResponse {
            release: ReleaseSummary {
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
            },
            content: "poll_interval_ms: 5000\n".to_owned(),
            diff_summary: Some(serde_json::json!({"fields_changed": 1})),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "release": {
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
                },
                "content": "poll_interval_ms: 5000\n",
                "diff_summary": {
                    "fields_changed": 1
                }
            })
        );
    }
}
