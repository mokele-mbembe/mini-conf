use crate::config::AppConfig;
use infra::AppIdentity;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct AppState {
    identity: AppIdentity,
    config: AppConfig,
    db_pool: Option<PgPool>,
}

impl AppState {
    pub fn new(identity: AppIdentity, config: AppConfig, db_pool: Option<PgPool>) -> Self {
        Self {
            identity,
            config,
            db_pool,
        }
    }

    pub const fn identity(&self) -> AppIdentity {
        self.identity
    }

    pub const fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn db_pool(&self) -> Option<&PgPool> {
        self.db_pool.as_ref()
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
            None,
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
            None,
        );

        assert_eq!(
            state.config(),
            &AppConfig {
                app_env: AppEnv::Dev,
                http_addr: "0.0.0.0:8080".to_owned(),
                database_url: "postgres://127.0.0.1:5432/postgres".to_owned(),
                init_db_on_boot: false,
                init_admin_username: None,
                init_admin_password: None,
                init_users_file: None,
                static_dir: PathBuf::from("apps/web/dist"),
                openapi_export_path: PathBuf::from("docs/artifacts/openapi.json"),
            }
        );
    }

    #[test]
    fn exposes_absent_database_pool_before_bootstrap_connects() {
        let state = AppState::new(
            AppIdentity::new("mini-conf-server", "0.1.0"),
            AppConfig::default(),
            None,
        );

        assert!(state.db_pool().is_none());
    }
}
