use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CloneSourceAvailability {
    pub draft: bool,
    pub latest_release: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CloneSourceSummary {
    pub deployment_instance_id: i64,
    pub deployment_key: String,
    pub name: String,
    pub environment_id: i64,
    pub environment_name: String,
    pub is_template: bool,
    pub available_sources: CloneSourceAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CloneSourceListResponse {
    pub items: Vec<CloneSourceSummary>,
    pub next_cursor: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_source_list_response_serializes_expected_shape() {
        let response = CloneSourceListResponse {
            items: vec![CloneSourceSummary {
                deployment_instance_id: 1,
                deployment_key: "store-001".to_string(),
                name: "Store 001".to_string(),
                environment_id: 2,
                environment_name: "Production".to_string(),
                is_template: false,
                available_sources: CloneSourceAvailability {
                    draft: true,
                    latest_release: false,
                },
            }],
            next_cursor: Some(42),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["items"][0]["deployment_instance_id"], 1);
        assert_eq!(json["items"][0]["deployment_key"], "store-001");
        assert_eq!(json["items"][0]["available_sources"]["draft"], true);
        assert_eq!(
            json["items"][0]["available_sources"]["latest_release"],
            false
        );
        assert_eq!(json["next_cursor"], 42);
    }
}
