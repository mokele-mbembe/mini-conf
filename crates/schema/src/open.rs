use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveConfigResponse {
    pub project: String,
    pub environment: String,
    pub deployment: ResolveDeployment,
    pub config: String,
    pub release: ResolveRelease,
    pub fetch: ResolveFetch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveDeployment {
    pub key: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveRelease {
    pub revision: String,
    pub content_hash: String,
    pub format: String,
    pub published_at: String,
    pub apply_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveFetch {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseContentResponse {
    pub release: ResolveRelease,
    pub deployment: ReleaseDeployment,
    pub config: ReleaseConfig,
    pub content: String,
    pub metadata: ReleaseMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseDeployment {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseMetadata {
    pub schema_version: Option<String>,
    pub change_summary: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        ReleaseConfig, ReleaseContentResponse, ReleaseDeployment, ReleaseMetadata,
        ResolveConfigResponse, ResolveDeployment, ResolveFetch, ResolveRelease,
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
                schema_version: Some("v1".to_owned()),
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
                    "schema_version": "v1",
                    "change_summary": "adjust polling interval"
                }
            })
        );
    }
}
