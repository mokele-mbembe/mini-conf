use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SetupStatusResponse {
    pub setup_required: bool,
    pub setup_completed_at: Option<String>,
    pub setup_completed_by_user_id: Option<i64>,
    pub active_platform_admin_count: i64,
    pub project_count: i64,
}

#[cfg(test)]
mod tests {
    use super::SetupStatusResponse;

    #[test]
    fn setup_status_response_serializes_expected_shape() {
        let value = serde_json::to_value(SetupStatusResponse {
            setup_required: true,
            setup_completed_at: None,
            setup_completed_by_user_id: None,
            active_platform_admin_count: 1,
            project_count: 0,
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "setup_required": true,
                "setup_completed_at": null,
                "setup_completed_by_user_id": null,
                "active_platform_admin_count": 1,
                "project_count": 0
            })
        );
    }
}
