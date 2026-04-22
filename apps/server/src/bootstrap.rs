use crate::{
    app,
    auth::hash_password,
    config::{AppConfig, ConfigError},
    state::AppState,
};
use infra::AppIdentity;
use serde::Deserialize;
use sqlx::{PgPool, migrate::Migrator};
use std::{fmt, fs, path::Path};
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt as tracing_fmt};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

#[derive(Debug, Deserialize)]
struct UserSeedFile {
    users: Vec<UserSeedEntry>,
}

#[derive(Debug, Deserialize)]
struct UserSeedEntry {
    username: String,
    password: Option<String>,
    password_hash: Option<String>,
    status: Option<String>,
    is_platform_admin: Option<bool>,
    must_change_password: Option<bool>,
    memberships: Option<Vec<ProjectMembershipSeed>>,
}

#[derive(Debug, Deserialize)]
struct ProjectMembershipSeed {
    project_id: Option<i64>,
    project_code: Option<String>,
    role: String,
}

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
        seed_users_from_file_if_configured(&pool, &config).await?;

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
        INSERT INTO users (
            username,
            password_hash,
            status,
            is_platform_admin,
            must_change_password,
            password_updated_at
        )
        VALUES ($1, $2, 'active', TRUE, FALSE, NOW())
        ON CONFLICT (username)
        DO NOTHING
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_users_from_file_if_configured(
    pool: &PgPool,
    config: &AppConfig,
) -> Result<(), StartupError> {
    let Some(path) = config.init_users_file.as_deref() else {
        return Ok(());
    };

    tracing::info!(path = %path.display(), "seeding bootstrap users from file");
    let seed_file = load_user_seed_file(path)?;

    for user in seed_file.users {
        seed_user_entry(pool, user).await?;
    }

    Ok(())
}

fn load_user_seed_file(path: &Path) -> Result<UserSeedFile, StartupError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            format!("failed to read user seed file: {error}"),
        ))
    })?;

    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    let parsed = match extension.as_deref() {
        Some("yaml") | Some("yml") => serde_yaml::from_str(&raw).map_err(|error| error.to_string()),
        Some("json") => serde_json::from_str(&raw).map_err(|error| error.to_string()),
        _ => serde_json::from_str(&raw)
            .map_err(|json_error| json_error.to_string())
            .or_else(|_| serde_yaml::from_str(&raw).map_err(|yaml_error| yaml_error.to_string())),
    };

    parsed.map_err(|error| {
        StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            format!("failed to parse user seed file: {error}"),
        ))
    })
}

async fn seed_user_entry(pool: &PgPool, user: UserSeedEntry) -> Result<(), StartupError> {
    let username = non_empty_seed("INIT_USERS_FILE", "username", &user.username)?;
    let status = validate_user_status(user.status.as_deref())?;
    let password_hash = resolve_user_password_hash(&user)?;
    let is_platform_admin = user.is_platform_admin.unwrap_or(false);
    let must_change_password = user.must_change_password.unwrap_or(false);

    let inserted_user_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO users (
            username,
            password_hash,
            status,
            is_platform_admin,
            must_change_password,
            password_updated_at
        )
        VALUES ($1, $2, $3, $4, $5, NOW())
        ON CONFLICT (username)
        DO NOTHING
        RETURNING id
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .bind(status)
    .bind(is_platform_admin)
    .bind(must_change_password)
    .fetch_optional(pool)
    .await?;

    let user_id: i64 = if let Some(id) = inserted_user_id {
        id
    } else {
        sqlx::query_scalar("SELECT id FROM users WHERE username = $1 LIMIT 1")
            .bind(username)
            .fetch_one(pool)
            .await?
    };

    for membership in user.memberships.unwrap_or_default() {
        let role = validate_project_role(&membership.role)?;
        let project_id = resolve_seed_project_id(pool, membership).await?;

        sqlx::query(
            r#"
            INSERT INTO project_members (project_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (project_id, user_id)
            DO NOTHING
            "#,
        )
        .bind(project_id)
        .bind(user_id)
        .bind(role)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn resolve_user_password_hash(user: &UserSeedEntry) -> Result<String, StartupError> {
    match (user.password.as_deref(), user.password_hash.as_deref()) {
        (Some(password), None) => hash_password(password).map_err(|_| {
            StartupError::Config(ConfigError::from_seed(
                "INIT_USERS_FILE",
                "failed to hash seeded user password",
            ))
        }),
        (None, Some(password_hash)) => Ok(password_hash.to_owned()),
        (Some(_), Some(_)) => Err(StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            "seeded user must specify either password or password_hash, not both",
        ))),
        (None, None) => Err(StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            "seeded user must specify password or password_hash",
        ))),
    }
}

fn validate_user_status(value: Option<&str>) -> Result<&'static str, StartupError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("active") => Ok("active"),
        Some("disabled") => Ok("disabled"),
        Some(_) => Err(StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            "seeded user status must be active or disabled",
        ))),
    }
}

fn validate_project_role(value: &str) -> Result<&'static str, StartupError> {
    match value.trim() {
        "admin" => Ok("admin"),
        "editor" => Ok("editor"),
        "viewer" => Ok("viewer"),
        _ => Err(StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            "seeded project member role must be admin, editor, or viewer",
        ))),
    }
}

async fn resolve_seed_project_id(
    pool: &PgPool,
    membership: ProjectMembershipSeed,
) -> Result<i64, StartupError> {
    match (
        membership.project_id,
        membership
            .project_code
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) {
        (Some(project_id), None) => {
            sqlx::query_scalar::<_, i64>("SELECT id FROM projects WHERE id = $1 LIMIT 1")
                .bind(project_id)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| {
                    StartupError::Config(ConfigError::from_seed(
                        "INIT_USERS_FILE",
                        "seeded project membership references an unknown project_id",
                    ))
                })
        }
        (None, Some(project_code)) => {
            sqlx::query_scalar::<_, i64>("SELECT id FROM projects WHERE code = $1 LIMIT 1")
                .bind(project_code)
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| {
                    StartupError::Config(ConfigError::from_seed(
                        "INIT_USERS_FILE",
                        "seeded project membership references an unknown project_code",
                    ))
                })
        }
        _ => Err(StartupError::Config(ConfigError::from_seed(
            "INIT_USERS_FILE",
            "seeded project membership must specify exactly one of project_id or project_code",
        ))),
    }
}

fn non_empty_seed<'a>(
    field: &'static str,
    label: &'static str,
    value: &'a str,
) -> Result<&'a str, StartupError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(StartupError::Config(ConfigError::from_seed(
            field,
            format!("seeded user {label} must not be empty"),
        )))
    } else {
        Ok(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::{StartupError, build_state, run};
    use crate::config::AppConfig;

    #[test]
    fn startup_error_wraps_configuration_failures() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("qa".to_owned()),
            _ => None,
        })
        .map_err(StartupError::from)
        .expect_err("config should fail");

        assert_eq!(
            error.to_string(),
            "configuration error: APP_ENV: unsupported APP_ENV value: qa"
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
