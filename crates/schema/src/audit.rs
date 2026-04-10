use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = audit_log_summary_example)]
pub struct AuditLogSummary {
    pub id: i64,
    pub project_id: Option<i64>,
    pub user_id: Option<i64>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub detail: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = audit_log_list_response_example)]
pub struct AuditLogListResponse {
    pub items: Vec<AuditLogSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = deployment_sync_record_summary_example)]
pub struct DeploymentSyncRecordSummary {
    pub id: i64,
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub config_file_id: Option<i64>,
    pub release_id: Option<i64>,
    pub process_key: Option<String>,
    pub revision: Option<String>,
    pub action: String,
    pub status: String,
    pub message: Option<String>,
    pub detail: Option<Value>,
    pub reported_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = deployment_sync_record_list_response_example)]
pub struct DeploymentSyncRecordListResponse {
    pub items: Vec<DeploymentSyncRecordSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = deployment_heartbeat_summary_example)]
pub struct DeploymentHeartbeatSummary {
    pub id: i64,
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub process_key: String,
    pub metadata: Option<Value>,
    pub reported_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(example = deployment_heartbeat_list_response_example)]
pub struct DeploymentHeartbeatListResponse {
    pub items: Vec<DeploymentHeartbeatSummary>,
}

fn audit_log_summary_example() -> serde_json::Value {
    serde_json::json!({
        "id": 41,
        "project_id": 7,
        "user_id": 1,
        "action": "project_member.created",
        "resource_type": "project_member",
        "resource_id": "17",
        "detail": {
            "username": "alice",
            "role": "viewer"
        },
        "created_at": "2026-04-10T12:00:00Z"
    })
}

fn audit_log_list_response_example() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": 41,
                "project_id": 7,
                "user_id": 1,
                "action": "project_member.created",
                "resource_type": "project_member",
                "resource_id": "17",
                "detail": {
                    "username": "alice",
                    "role": "viewer"
                },
                "created_at": "2026-04-10T12:00:00Z"
            }
        ]
    })
}

fn deployment_sync_record_summary_example() -> serde_json::Value {
    serde_json::json!({
        "id": 88,
        "project_id": 7,
        "deployment_instance_id": 3,
        "config_file_id": 5,
        "release_id": 8,
        "process_key": "main",
        "revision": "20260410.0001",
        "action": "apply",
        "status": "success",
        "message": "config applied",
        "detail": {
            "duration_ms": 87
        },
        "reported_at": "2026-04-10T12:00:00Z"
    })
}

fn deployment_sync_record_list_response_example() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": 88,
                "project_id": 7,
                "deployment_instance_id": 3,
                "config_file_id": 5,
                "release_id": 8,
                "process_key": "main",
                "revision": "20260410.0001",
                "action": "apply",
                "status": "success",
                "message": "config applied",
                "detail": {
                    "duration_ms": 87
                },
                "reported_at": "2026-04-10T12:00:00Z"
            }
        ]
    })
}

fn deployment_heartbeat_summary_example() -> serde_json::Value {
    serde_json::json!({
        "id": 21,
        "project_id": 7,
        "deployment_instance_id": 3,
        "process_key": "vision",
        "metadata": {
            "ip": "10.0.0.8",
            "version": "1.0.3"
        },
        "reported_at": "2026-04-10T12:05:00Z"
    })
}

fn deployment_heartbeat_list_response_example() -> serde_json::Value {
    serde_json::json!({
        "items": [
            {
                "id": 21,
                "project_id": 7,
                "deployment_instance_id": 3,
                "process_key": "vision",
                "metadata": {
                    "ip": "10.0.0.8",
                    "version": "1.0.3"
                },
                "reported_at": "2026-04-10T12:05:00Z"
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AuditLogListResponse, AuditLogSummary, DeploymentHeartbeatListResponse,
        DeploymentHeartbeatSummary, DeploymentSyncRecordListResponse, DeploymentSyncRecordSummary,
    };

    #[test]
    fn audit_log_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(AuditLogListResponse {
            items: vec![AuditLogSummary {
                id: 1,
                project_id: Some(7),
                user_id: Some(9),
                action: "project_member.created".to_owned(),
                resource_type: "project_member".to_owned(),
                resource_id: "17".to_owned(),
                detail: Some(serde_json::json!({"role": "viewer"})),
                created_at: "2026-04-10T12:00:00Z".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "project_id": 7,
                        "user_id": 9,
                        "action": "project_member.created",
                        "resource_type": "project_member",
                        "resource_id": "17",
                        "detail": {
                            "role": "viewer"
                        },
                        "created_at": "2026-04-10T12:00:00Z"
                    }
                ]
            })
        );
    }

    #[test]
    fn deployment_sync_record_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentSyncRecordListResponse {
            items: vec![DeploymentSyncRecordSummary {
                id: 1,
                project_id: 7,
                deployment_instance_id: 3,
                config_file_id: Some(5),
                release_id: Some(8),
                process_key: Some("main".to_owned()),
                revision: Some("20260410.0001".to_owned()),
                action: "apply".to_owned(),
                status: "success".to_owned(),
                message: Some("config applied".to_owned()),
                detail: Some(serde_json::json!({"duration_ms": 87})),
                reported_at: "2026-04-10T12:00:00Z".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 1,
                        "project_id": 7,
                        "deployment_instance_id": 3,
                        "config_file_id": 5,
                        "release_id": 8,
                        "process_key": "main",
                        "revision": "20260410.0001",
                        "action": "apply",
                        "status": "success",
                        "message": "config applied",
                        "detail": {
                            "duration_ms": 87
                        },
                        "reported_at": "2026-04-10T12:00:00Z"
                    }
                ]
            })
        );
    }

    #[test]
    fn deployment_heartbeat_list_response_serializes_expected_shape() {
        let value = serde_json::to_value(DeploymentHeartbeatListResponse {
            items: vec![DeploymentHeartbeatSummary {
                id: 21,
                project_id: 7,
                deployment_instance_id: 3,
                process_key: "vision".to_owned(),
                metadata: Some(serde_json::json!({"ip": "10.0.0.8"})),
                reported_at: "2026-04-10T12:05:00Z".to_owned(),
            }],
        })
        .expect("response should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "items": [
                    {
                        "id": 21,
                        "project_id": 7,
                        "deployment_instance_id": 3,
                        "process_key": "vision",
                        "metadata": {
                            "ip": "10.0.0.8"
                        },
                        "reported_at": "2026-04-10T12:05:00Z"
                    }
                ]
            })
        );
    }
}
