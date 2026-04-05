use crate::config::AppConfig;
use infra::AppIdentity;

#[derive(Debug, Clone)]
pub struct AppState {
    identity: AppIdentity,
    config: AppConfig,
}

impl AppState {
    pub fn new(identity: AppIdentity, config: AppConfig) -> Self {
        Self { identity, config }
    }

    pub const fn identity(&self) -> AppIdentity {
        self.identity
    }

    pub const fn config(&self) -> &AppConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::AppState;
    use crate::config::{AppConfig, AppEnv};
    use infra::AppIdentity;
    use std::path::PathBuf;

    #[test]
    fn exposes_app_identity() {
        let state = AppState::new(
            AppIdentity::new("mini-conf-server", "0.1.0"),
            AppConfig::default(),
        );

        assert_eq!(
            state.identity(),
            AppIdentity::new("mini-conf-server", "0.1.0")
        );
    }

    #[test]
    fn exposes_loaded_app_config() {
        let state = AppState::new(
            AppIdentity::new("mini-conf-server", "0.1.0"),
            AppConfig::default(),
        );

        assert_eq!(
            state.config(),
            &AppConfig {
                app_env: AppEnv::Dev,
                http_addr: "0.0.0.0:8080".to_owned(),
                database_url: "postgres://mini_conf:secret@127.0.0.1:5432/mini_conf".to_owned(),
                static_dir: PathBuf::from("apps/web/dist"),
                openapi_export_path: PathBuf::from("docs/openapi/openapi.json"),
            }
        );
    }
}
