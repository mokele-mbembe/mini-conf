use domain::health::HealthStatus;
use infra::AppIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthzResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

impl HealthzResponse {
    pub fn ok(identity: AppIdentity) -> Self {
        Self {
            status: HealthStatus::Ok.as_str().to_owned(),
            service: identity.service_name.to_owned(),
            version: identity.version.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HealthzResponse;
    use infra::AppIdentity;

    #[test]
    fn builds_ok_response_from_app_identity() {
        assert_eq!(
            HealthzResponse::ok(AppIdentity::new("mini-conf-server", "0.1.0")),
            HealthzResponse {
                status: "ok".to_owned(),
                service: "mini-conf-server".to_owned(),
                version: "0.1.0".to_owned(),
            }
        );
    }
}
