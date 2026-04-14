use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolveConfigResponse {
    pub project: String,
    pub environment: String,
    pub deployment: ResolveDeployment,
    pub config: String,
    pub release: ResolveRelease,
    pub fetch: ResolveFetch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolveDeployment {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolveRelease {
    pub revision: String,
    pub content_hash: String,
    pub format: String,
    pub published_at: String,
    pub apply_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResolveFetch {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseContentResponse {
    pub release: ResolveRelease,
    pub deployment: ReleaseDeployment,
    pub config: ReleaseConfig,
    pub content: String,
    pub metadata: ReleaseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseDeployment {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReleaseMetadata {
    pub change_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConfigBundleResponse {
    pub project: String,
    pub environment: String,
    pub deployment: ResolveDeployment,
    pub configs: Vec<ConfigBundleItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ConfigBundleItem {
    pub config: String,
    pub revision: String,
    pub content_hash: String,
    pub format: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DeploymentSyncResponse {
    pub ok: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        ConfigBundleItem, ConfigBundleResponse, DeploymentSyncResponse, ReleaseConfig,
        ReleaseContentResponse, ReleaseDeployment, ReleaseMetadata, ResolveConfigResponse,
        ResolveDeployment, ResolveFetch, ResolveRelease,
    };

    #[test]
    fn resolve_response_serializes_expected_shape() {
        let response = ResolveConfigResponse {
            project: "coffee-legacy".to_owned(),
            environment: "prod".to_owned(),
            deployment: ResolveDeployment {
                key: "store-001".to_owned(),
                name: "Store 001".to_owned(),
            },
            config: "main".to_owned(),
            release: ResolveRelease {
                revision: "20260405.0001".to_owned(),
                content_hash: "abc123".to_owned(),
                format: "yaml".to_owned(),
                published_at: "2026-04-05T12:00:00Z".to_owned(),
                apply_mode: "soft".to_owned(),
            },
            fetch: ResolveFetch {
                url: "/api/open/releases/20260405.0001".to_owned(),
            },
        };

        let value = serde_json::to_value(&response).expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "project": "coffee-legacy",
                "environment": "prod",
                "deployment": {
                    "key": "store-001",
                    "name": "Store 001"
                },
                "config": "main",
                "release": {
                    "revision": "20260405.0001",
                    "content_hash": "abc123",
                    "format": "yaml",
                    "published_at": "2026-04-05T12:00:00Z",
                    "apply_mode": "soft"
                },
                "fetch": {
                    "url": "/api/open/releases/20260405.0001"
                }
            })
        );
    }

    #[test]
    fn release_response_serializes_expected_shape() {
        let response = ReleaseContentResponse {
            release: ResolveRelease {
                revision: "20260405.0001".to_owned(),
                content_hash: "abc123".to_owned(),
                format: "yaml".to_owned(),
                published_at: "2026-04-05T12:00:00Z".to_owned(),
                apply_mode: "soft".to_owned(),
            },
            deployment: ReleaseDeployment {
                project: "coffee-legacy".to_owned(),
                environment: "prod".to_owned(),
                deployment_key: "store-001".to_owned(),
            },
            config: ReleaseConfig {
                name: "main".to_owned(),
            },
            content: "log_level: info\n".to_owned(),
            metadata: ReleaseMetadata {
                change_summary: Some("adjust polling interval".to_owned()),
            },
        };

        let value = serde_json::to_value(&response).expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "release": {
                    "revision": "20260405.0001",
                    "content_hash": "abc123",
                    "format": "yaml",
                    "published_at": "2026-04-05T12:00:00Z",
                    "apply_mode": "soft"
                },
                "deployment": {
                    "project": "coffee-legacy",
                    "environment": "prod",
                    "deployment_key": "store-001"
                },
                "config": {
                    "name": "main"
                },
                "content": "log_level: info\n",
                "metadata": {
                    "change_summary": "adjust polling interval"
                }
            })
        );
    }

    #[test]
    fn config_bundle_response_serializes_expected_shape() {
        let response = ConfigBundleResponse {
            project: "coffee-legacy".to_owned(),
            environment: "prod".to_owned(),
            deployment: ResolveDeployment {
                key: "store-001".to_owned(),
                name: "Store 001".to_owned(),
            },
            configs: vec![
                ConfigBundleItem {
                    config: "main".to_owned(),
                    revision: "20260405.0001".to_owned(),
                    content_hash: "aaa".to_owned(),
                    format: "yaml".to_owned(),
                    content: "log_level: info\n".to_owned(),
                },
                ConfigBundleItem {
                    config: "vision".to_owned(),
                    revision: "20260405.0002".to_owned(),
                    content_hash: "bbb".to_owned(),
                    format: "yaml".to_owned(),
                    content: "camera_enabled: true\n".to_owned(),
                },
            ],
        };

        let value = serde_json::to_value(&response).expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "project": "coffee-legacy",
                "environment": "prod",
                "deployment": {
                    "key": "store-001",
                    "name": "Store 001"
                },
                "configs": [
                    {
                        "config": "main",
                        "revision": "20260405.0001",
                        "content_hash": "aaa",
                        "format": "yaml",
                        "content": "log_level: info\n"
                    },
                    {
                        "config": "vision",
                        "revision": "20260405.0002",
                        "content_hash": "bbb",
                        "format": "yaml",
                        "content": "camera_enabled: true\n"
                    }
                ]
            })
        );
    }

    #[test]
    fn deployment_sync_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentSyncResponse { ok: true })
            .expect("response should serialize");

        assert_eq!(value, serde_json::json!({ "ok": true }));
    }
}
