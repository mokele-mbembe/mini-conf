use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::draft::DraftResponse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SavedVersionSummary {
    pub id: i64,
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub config_file_id: i64,
    pub title: String,
    pub note: Option<String>,
    pub format: String,
    pub source_draft_version: i64,
    pub created_by: i64,
    pub created_by_username: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SavedVersionListResponse {
    pub items: Vec<SavedVersionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SavedVersionDetail {
    pub id: i64,
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub config_file_id: i64,
    pub title: String,
    pub note: Option<String>,
    pub content: String,
    pub content_hash: String,
    pub format: String,
    pub source_draft_version: i64,
    pub created_by: i64,
    pub created_by_username: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SavedVersionDetailResponse {
    pub saved_version: SavedVersionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SavedVersionRestoreResponse {
    pub draft: DraftResponse,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft::DraftResponse;

    #[test]
    fn saved_version_summary_serializes_expected_shape() {
        let value = serde_json::to_value(SavedVersionSummary {
            id: 301,
            project_id: 1,
            deployment_instance_id: 18,
            config_file_id: 7,
            title: "2026-04-20 18:42".to_owned(),
            note: Some("门店调试版".to_owned()),
            format: "yaml".to_owned(),
            source_draft_version: 12,
            created_by: 9,
            created_by_username: "alice".to_owned(),
            created_at: "2026-04-20T10:42:00Z".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "id": 301,
                "project_id": 1,
                "deployment_instance_id": 18,
                "config_file_id": 7,
                "title": "2026-04-20 18:42",
                "note": "门店调试版",
                "format": "yaml",
                "source_draft_version": 12,
                "created_by": 9,
                "created_by_username": "alice",
                "created_at": "2026-04-20T10:42:00Z"
            })
        );
    }

    #[test]
    fn saved_version_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(SavedVersionListResponse {
            items: vec![SavedVersionSummary {
                id: 301,
                project_id: 1,
                deployment_instance_id: 18,
                config_file_id: 7,
                title: "2026-04-20 18:42".to_owned(),
                note: None,
                format: "yaml".to_owned(),
                source_draft_version: 12,
                created_by: 9,
                created_by_username: "alice".to_owned(),
                created_at: "2026-04-20T10:42:00Z".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 301,
                        "project_id": 1,
                        "deployment_instance_id": 18,
                        "config_file_id": 7,
                        "title": "2026-04-20 18:42",
                        "note": null,
                        "format": "yaml",
                        "source_draft_version": 12,
                        "created_by": 9,
                        "created_by_username": "alice",
                        "created_at": "2026-04-20T10:42:00Z"
                    }
                ]
            })
        );
    }

    #[test]
    fn saved_version_detail_response_serializes_expected_shape() {
        let value = serde_json::to_value(SavedVersionDetailResponse {
            saved_version: SavedVersionDetail {
                id: 301,
                project_id: 1,
                deployment_instance_id: 18,
                config_file_id: 7,
                title: "2026-04-20 18:42".to_owned(),
                note: Some("门店调试版".to_owned()),
                content: "shop:\n  id: store-001\n".to_owned(),
                content_hash: "a".repeat(64),
                format: "yaml".to_owned(),
                source_draft_version: 12,
                created_by: 9,
                created_by_username: "alice".to_owned(),
                created_at: "2026-04-20T10:42:00Z".to_owned(),
            },
        })
        .expect("response should serialize");

        let sv = value
            .get("saved_version")
            .expect("should have saved_version key");
        assert_eq!(sv.get("id").and_then(|v| v.as_i64()), Some(301));
        assert_eq!(
            sv.get("content").and_then(|v| v.as_str()),
            Some("shop:\n  id: store-001\n")
        );
        assert_eq!(sv.get("note").and_then(|v| v.as_str()), Some("门店调试版"));
    }

    #[test]
    fn saved_version_restore_response_serializes_expected_shape() {
        let value = serde_json::to_value(SavedVersionRestoreResponse {
            draft: DraftResponse {
                deployment_instance_id: 18,
                config_file_id: 7,
                format: "yaml".to_owned(),
                content: "shop:\n  id: store-001\n".to_owned(),
                version: 13,
                updated_at: "2026-04-20T10:50:00Z".to_owned(),
            },
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "draft": {
                    "deployment_instance_id": 18,
                    "config_file_id": 7,
                    "format": "yaml",
                    "content": "shop:\n  id: store-001\n",
                    "version": 13,
                    "updated_at": "2026-04-20T10:50:00Z"
                }
            })
        );
    }
}
