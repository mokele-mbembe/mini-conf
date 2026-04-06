use crate::{
    app,
    auth::hash_password,
    config::{AppConfig, ConfigError},
    state::AppState,
};
use infra::AppIdentity;
use sqlx::{PgPool, migrate::Migrator};
use std::fmt;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt as tracing_fmt};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug)]
pub enum StartupError {
    Config(ConfigError),
    Database(sqlx::Error),
    Migrate(sqlx::migrate::MigrateError),
    Io(std::io::Error),
}

impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(f, "configuration error: {error}"),
            Self::Database(error) => write!(f, "database connection error: {error}"),
            Self::Migrate(error) => write!(f, "database migration error: {error}"),
            Self::Io(error) => write!(f, "server io error: {error}"),
        }
    }
}

impl std::error::Error for StartupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Database(error) => Some(error),
            Self::Migrate(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<ConfigError> for StartupError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<sqlx::Error> for StartupError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

impl From<sqlx::migrate::MigrateError> for StartupError {
    fn from(value: sqlx::migrate::MigrateError) -> Self {
        Self::Migrate(value)
    }
}

impl From<std::io::Error> for StartupError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

pub async fn build_state(config: AppConfig) -> Result<AppState, StartupError> {
    let db_pool: Option<PgPool> = if config.init_db_on_boot {
        tracing::info!("database bootstrap enabled; connecting to postgres");

        let pool = infra::db::connect(&config.database_url).await?;

        tracing::info!("database connected; applying migrations");
        MIGRATOR.run(&pool).await?;
        tracing::info!("database migrations applied");
        seed_admin_if_configured(&pool, &config).await?;

        Some(pool)
    } else {
        tracing::info!("database bootstrap disabled; skipping postgres connect and migrations");
        None
    };

    Ok(AppState::new(
        AppIdentity::new("mini-conf-server", env!("CARGO_PKG_VERSION")),
        config,
        db_pool,
    ))
}

pub async fn run(state: AppState) -> Result<(), StartupError> {
    let listener = TcpListener::bind(&state.config().http_addr).await?;

    tracing::info!(
        address = %state.config().http_addr,
        env = state.config().app_env.as_str(),
        db_boot_enabled = state.config().init_db_on_boot,
        db_connected = state.db_pool().is_some(),
        "starting mini-conf server"
    );

    axum::serve(listener, app(state)).await?;

    Ok(())
}

async fn seed_admin_if_configured(pool: &PgPool, config: &AppConfig) -> Result<(), StartupError> {
    let (Some(username), Some(password)) = (
        config.init_admin_username.as_deref(),
        config.init_admin_password.as_deref(),
    ) else {
        return Ok(());
    };

    tracing::info!(username, "seeding bootstrap admin user");
    let password_hash = hash_password(password).map_err(|_| {
        StartupError::Config(ConfigError::from_seed(
            "INIT_ADMIN_PASSWORD",
            "failed to hash admin password",
        ))
    })?;

    sqlx::query(
        r#"
        INSERT INTO users (username, password_hash, status)
        VALUES ($1, $2, 'active')
        ON CONFLICT (username)
        DO UPDATE SET
            password_hash = EXCLUDED.password_hash,
            status = 'active',
            updated_at = NOW()
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{StartupError, build_state, run};
    use crate::config::AppConfig;

    #[test]
    fn startup_error_wraps_configuration_failures() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("staging".to_owned()),
            _ => None,
        })
        .map_err(StartupError::from)
        .expect_err("config should fail");

        assert_eq!(
            error.to_string(),
            "configuration error: APP_ENV: unsupported APP_ENV value: staging"
        );
    }

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
        assert!(
            matches!(result, Err(StartupError::Database(_))),
            "invalid url should surface as a database startup error"
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
