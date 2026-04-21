use crate::open::ConfigBundleResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInstanceSummary {
    pub id: i64,
    pub deployment_uid: String,
    pub project_id: i64,
    pub environment_id: i64,
    pub environment_code: String,
    pub environment_name: String,
    pub deployment_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_template: bool,
    pub template_source_id: Option<i64>,
    pub status: String,
    pub is_archived: bool,
    pub archived_at: Option<String>,
    pub archived_by: Option<i64>,
    pub archive_reason: Option<String>,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<i64>,
    pub delete_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentInstanceListResponse {
    pub items: Vec<DeploymentInstanceSummary>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentPreviewItem {
    pub config_file_id: i64,
    pub code: String,
    pub name: String,
    pub is_required: bool,
    pub source: String,
    pub status: String,
    pub format: String,
    pub content: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentBundlePreviewResponse {
    pub deployment_instance_id: i64,
    pub items: Vec<DeploymentPreviewItem>,
    pub open_bundle_preview: ConfigBundleResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentTokenResetResponse {
    pub deployment_instance_id: i64,
    pub credential_name: String,
    pub token_preview: String,
    pub token: String,
}

#[cfg(test)]
mod tests {
    use super::{
        DeploymentBundlePreviewResponse, DeploymentInstanceListResponse, DeploymentInstanceSummary,
        DeploymentPreviewItem, DeploymentTokenResetResponse,
    };
    use crate::open::{ConfigBundleItem, ConfigBundleResponse, ResolveDeployment};

    #[test]
    fn deployment_instance_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentInstanceListResponse {
            items: vec![DeploymentInstanceSummary {
                id: 1,
                deployment_uid: "4f6f2c18-e0c8-46ca-9754-2911e7bc0ba8".to_owned(),
                project_id: 7,
                environment_id: 3,
                environment_code: "prod".to_owned(),
                environment_name: "Production".to_owned(),
                deployment_key: "store-001".to_owned(),
                name: "Store 001".to_owned(),
                description: Some("Hangzhou store 001".to_owned()),
                is_template: false,
                template_source_id: None,
                status: "active".to_owned(),
                is_archived: false,
                archived_at: None,
                archived_by: None,
                archive_reason: None,
                deleted_at: None,
                deleted_by: None,
                delete_reason: None,
            }],
            total: 1,
            page: 1,
            page_size: 20,
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "deployment_uid": "4f6f2c18-e0c8-46ca-9754-2911e7bc0ba8",
                        "project_id": 7,
                        "environment_id": 3,
                        "environment_code": "prod",
                        "environment_name": "Production",
                        "deployment_key": "store-001",
                        "name": "Store 001",
                        "description": "Hangzhou store 001",
                        "is_template": false,
                        "template_source_id": null,
                        "status": "active",
                        "is_archived": false,
                        "archived_at": null,
                        "archived_by": null,
                        "archive_reason": null,
                        "deleted_at": null,
                        "deleted_by": null,
                        "delete_reason": null
                    }
                ],
                "total": 1,
                "page": 1,
                "page_size": 20
            })
        );
    }

    #[test]
    fn deployment_bundle_preview_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentBundlePreviewResponse {
            deployment_instance_id: 9,
            items: vec![
                DeploymentPreviewItem {
                    config_file_id: 3,
                    code: "main".to_owned(),
                    name: "Main Config".to_owned(),
                    is_required: true,
                    source: "draft".to_owned(),
                    status: "ready".to_owned(),
                    format: "yaml".to_owned(),
                    content: Some("poll_interval_ms: 8000\n".to_owned()),
                    revision: Some("draft-v2".to_owned()),
                },
                DeploymentPreviewItem {
                    config_file_id: 4,
                    code: "vision".to_owned(),
                    name: "Vision Config".to_owned(),
                    is_required: false,
                    source: "none".to_owned(),
                    status: "missing_optional".to_owned(),
                    format: "yaml".to_owned(),
                    content: None,
                    revision: None,
                },
            ],
            open_bundle_preview: ConfigBundleResponse {
                project: "coffee-legacy".to_owned(),
                environment: "prod".to_owned(),
                deployment: ResolveDeployment {
                    key: "store-001".to_owned(),
                    name: "Store 001".to_owned(),
                },
                configs: vec![ConfigBundleItem {
                    config: "main".to_owned(),
                    revision: "draft-v2".to_owned(),
                    content_hash: "abc123".to_owned(),
                    format: "yaml".to_owned(),
                    content: "poll_interval_ms: 8000\n".to_owned(),
                }],
            },
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "deployment_instance_id": 9,
                "items": [
                    {
                        "config_file_id": 3,
                        "code": "main",
                        "name": "Main Config",
                        "is_required": true,
                        "source": "draft",
                        "status": "ready",
                        "format": "yaml",
                        "content": "poll_interval_ms: 8000\n",
                        "revision": "draft-v2"
                    },
                    {
                        "config_file_id": 4,
                        "code": "vision",
                        "name": "Vision Config",
                        "is_required": false,
                        "source": "none",
                        "status": "missing_optional",
                        "format": "yaml",
                        "content": null,
                        "revision": null
                    }
                ],
                "open_bundle_preview": {
                    "project": "coffee-legacy",
                    "environment": "prod",
                    "deployment": {
                        "key": "store-001",
                        "name": "Store 001"
                    },
                    "configs": [
                        {
                            "config": "main",
                            "revision": "draft-v2",
                            "content_hash": "abc123",
                            "format": "yaml",
                            "content": "poll_interval_ms: 8000\n"
                        }
                    ]
                }
            })
        );
    }

    #[test]
    fn deployment_token_reset_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentTokenResetResponse {
            deployment_instance_id: 9,
            credential_name: "default".to_owned(),
            token_preview: "mc_live_***".to_owned(),
            token: "mc_live_1234567890abcdef".to_owned(),
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "deployment_instance_id": 9,
                "credential_name": "default",
                "token_preview": "mc_live_***",
                "token": "mc_live_1234567890abcdef"
            })
        );
    }
}
