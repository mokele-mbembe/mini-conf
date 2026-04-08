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
pub struct ReleaseDiffSummary {
    pub is_initial: bool,
    pub has_changes: bool,
    pub added_lines: u32,
    pub removed_lines: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDetailResponse {
    pub release: ReleaseSummary,
    pub content: String,
    pub diff_summary: Option<ReleaseDiffSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDiffResponse {
    pub release: ReleaseSummary,
    pub base_release: Option<ReleaseSummary>,
    pub before_content: Option<String>,
    pub after_content: String,
    pub diff_summary: ReleaseDiffSummary,
}

#[cfg(test)]
mod tests {
    use super::{
        ReleaseDetailResponse, ReleaseDiffResponse, ReleaseDiffSummary, ReleaseListResponse,
        ReleaseSummary,
    };

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
            diff_summary: Some(ReleaseDiffSummary {
                is_initial: false,
                has_changes: true,
                added_lines: 1,
                removed_lines: 1,
            }),
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
                    "is_initial": false,
                    "has_changes": true,
                    "added_lines": 1,
                    "removed_lines": 1
                }
            })
        );
    }

    #[test]
    fn release_diff_response_serializes_expected_shape() {
        let value = serde_json::to_value(ReleaseDiffResponse {
            release: ReleaseSummary {
                id: 13,
                project_id: 1,
                deployment_instance_id: 8,
                config_file_id: 3,
                revision: "20260406.0002".to_owned(),
                content_hash: "def456".to_owned(),
                format: "yaml".to_owned(),
                change_summary: Some("bump poll interval".to_owned()),
                apply_mode: "soft".to_owned(),
                published_by: 1,
                published_at: "2026-04-06T12:05:00Z".to_owned(),
            },
            base_release: Some(ReleaseSummary {
                id: 12,
                project_id: 1,
                deployment_instance_id: 8,
                config_file_id: 3,
                revision: "20260406.0001".to_owned(),
                content_hash: "abc123".to_owned(),
                format: "yaml".to_owned(),
                change_summary: Some("initial".to_owned()),
                apply_mode: "soft".to_owned(),
                published_by: 1,
                published_at: "2026-04-06T12:00:00Z".to_owned(),
            }),
            before_content: Some("poll_interval_ms: 3000\n".to_owned()),
            after_content: "poll_interval_ms: 5000\n".to_owned(),
            diff_summary: ReleaseDiffSummary {
                is_initial: false,
                has_changes: true,
                added_lines: 1,
                removed_lines: 1,
            },
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "release": {
                    "id": 13,
                    "project_id": 1,
                    "deployment_instance_id": 8,
                    "config_file_id": 3,
                    "revision": "20260406.0002",
                    "content_hash": "def456",
                    "format": "yaml",
                    "change_summary": "bump poll interval",
                    "apply_mode": "soft",
                    "published_by": 1,
                    "published_at": "2026-04-06T12:05:00Z"
                },
                "base_release": {
                    "id": 12,
                    "project_id": 1,
                    "deployment_instance_id": 8,
                    "config_file_id": 3,
                    "revision": "20260406.0001",
                    "content_hash": "abc123",
                    "format": "yaml",
                    "change_summary": "initial",
                    "apply_mode": "soft",
                    "published_by": 1,
                    "published_at": "2026-04-06T12:00:00Z"
                },
                "before_content": "poll_interval_ms: 3000\n",
                "after_content": "poll_interval_ms: 5000\n",
                "diff_summary": {
                    "is_initial": false,
                    "has_changes": true,
                    "added_lines": 1,
                    "removed_lines": 1
                }
            })
        );
    }
}
