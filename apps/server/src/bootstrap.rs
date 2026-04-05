use crate::{app, config::AppConfig, state::AppState};
use infra::AppIdentity;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_target(false).init();
}

pub async fn build_state(config: AppConfig) -> Result<AppState, sqlx::Error> {
    let db_pool: Option<PgPool> = if config.init_db_on_boot {
        Some(infra::db::connect(&config.database_url).await?)
    } else {
        None
    };

    Ok(AppState::new(
        AppIdentity::new("mini-conf-server", env!("CARGO_PKG_VERSION")),
        config,
        db_pool,
    ))
}

pub async fn run(state: AppState) -> std::io::Result<()> {
    let listener = TcpListener::bind(&state.config().http_addr).await?;

    tracing::info!(
        address = %state.config().http_addr,
        env = state.config().app_env.as_str(),
        db_connected = state.db_pool().is_some(),
        "starting mini-conf server"
    );

    axum::serve(listener, app(state)).await
}

#[cfg(test)]
mod tests {
    use super::{build_state, run};
    use crate::config::AppConfig;

    #[tokio::test]
    async fn build_state_skips_database_connection_when_flag_is_disabled() {
        let state = build_state(AppConfig::default())
            .await
            .expect("state should build without connecting db");

        assert!(state.db_pool().is_none());
    }

    #[tokio::test]
    async fn build_state_returns_error_when_db_boot_is_enabled_with_invalid_url() {
        let config = AppConfig {
            init_db_on_boot: true,
            database_url: "not-a-postgres-url".to_owned(),
            ..AppConfig::default()
        };

        let result = build_state(config).await;

        assert!(
            result.is_err(),
            "state build should fail when db boot is enabled and url is invalid"
        );
    }

    #[tokio::test]
    async fn run_returns_error_when_http_addr_is_invalid() {
        let state = build_state(AppConfig {
            http_addr: "invalid-address".to_owned(),
            ..AppConfig::default()
        })
        .await
        .expect("state should build");

        let result = run(state).await;

        assert!(
            result.is_err(),
            "run should fail for an invalid bind address"
        );
    }
}
