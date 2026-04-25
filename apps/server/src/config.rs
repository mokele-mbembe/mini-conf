use std::{
    fmt,
    path::{Path, PathBuf},
};

const DEFAULT_HTTP_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_DATABASE_URL: &str = "postgres://127.0.0.1:5432/postgres";
const DEFAULT_STATIC_DIR: &str = "apps/web/dist";
const DEFAULT_OPENAPI_EXPORT_PATH: &str = "docs/artifacts/openapi.json";
const DEFAULT_INIT_DB_ON_BOOT: bool = false;
const DEFAULT_SESSION_COOKIE_SECURE: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Test,
    Staging,
    Prod,
}

impl AppEnv {
    fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Dev),
            "test" | "testing" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "prod" | "production" => Ok(Self::Prod),
            value => Err(ConfigError::invalid_env(value)),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Staging => "staging",
            Self::Prod => "prod",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub app_env: AppEnv,
    pub http_addr: String,
    pub database_url: String,
    pub init_db_on_boot: bool,
    pub session_cookie_secure: bool,
    pub init_admin_username: Option<String>,
    pub init_admin_password: Option<String>,
    pub init_users_file: Option<PathBuf>,
    pub static_dir: PathBuf,
    pub openapi_export_path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_env: AppEnv::Dev,
            http_addr: DEFAULT_HTTP_ADDR.to_owned(),
            database_url: DEFAULT_DATABASE_URL.to_owned(),
            init_db_on_boot: DEFAULT_INIT_DB_ON_BOOT,
            session_cookie_secure: DEFAULT_SESSION_COOKIE_SECURE,
            init_admin_username: None,
            init_admin_password: None,
            init_users_file: None,
            static_dir: PathBuf::from(DEFAULT_STATIC_DIR),
            openapi_export_path: PathBuf::from(DEFAULT_OPENAPI_EXPORT_PATH),
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup<F>(mut lookup: F) -> Result<Self, ConfigError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut config = Self::default();
        let mut database_url_was_set = false;

        if let Some(value) = lookup("APP_ENV") {
            config.app_env = AppEnv::parse(&value)?;
        }

        if let Some(value) = lookup("HTTP_ADDR") {
            config.http_addr = value;
        }

        if let Some(value) = lookup("DATABASE_URL") {
            config.database_url = value;
            database_url_was_set = true;
        }

        if let Some(value) = lookup("INIT_DB_ON_BOOT") {
            config.init_db_on_boot = parse_bool("INIT_DB_ON_BOOT", &value)?;
        }

        config.session_cookie_secure = matches!(config.app_env, AppEnv::Staging | AppEnv::Prod);

        if let Some(value) = lookup("SESSION_COOKIE_SECURE") {
            config.session_cookie_secure = parse_bool("SESSION_COOKIE_SECURE", &value)?;
        }

        if let Some(value) = lookup("INIT_ADMIN_USERNAME") {
            config.init_admin_username = non_empty(value);
        }

        if let Some(value) = lookup("INIT_ADMIN_PASSWORD") {
            config.init_admin_password = non_empty(value);
        }

        if let Some(value) = lookup("INIT_USERS_FILE") {
            config.init_users_file = non_empty(value).map(PathBuf::from);
        }

        if let Some(value) = lookup("STATIC_DIR") {
            config.static_dir = PathBuf::from(value);
        }

        if let Some(value) = lookup("OPENAPI_EXPORT_PATH") {
            config.openapi_export_path = PathBuf::from(value);
        }

        if config.init_db_on_boot && matches!(config.app_env, AppEnv::Staging | AppEnv::Prod) {
            return Err(ConfigError::unsupported_db_boot(config.app_env));
        }

        if matches!(config.app_env, AppEnv::Staging | AppEnv::Prod) && !database_url_was_set {
            return Err(ConfigError::missing_database_url(config.app_env));
        }

        Ok(config)
    }

    pub const fn should_connect_database_on_boot(&self) -> bool {
        self.init_db_on_boot || matches!(self.app_env, AppEnv::Staging | AppEnv::Prod)
    }

    pub fn static_dir(&self) -> &Path {
        &self.static_dir
    }

    pub fn openapi_export_path(&self) -> &Path {
        &self.openapi_export_path
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    field: &'static str,
    message: String,
}

impl ConfigError {
    fn invalid_env(value: &str) -> Self {
        Self {
            field: "APP_ENV",
            message: format!("unsupported APP_ENV value: {value}"),
        }
    }

    fn invalid_bool(field: &'static str, value: &str) -> Self {
        Self {
            field,
            message: format!("unsupported {field} value: {value}"),
        }
    }

    fn unsupported_db_boot(app_env: AppEnv) -> Self {
        Self {
            field: "INIT_DB_ON_BOOT",
            message: format!(
                "INIT_DB_ON_BOOT=true is only supported for APP_ENV=dev or APP_ENV=test, got {}",
                app_env.as_str()
            ),
        }
    }

    fn missing_database_url(app_env: AppEnv) -> Self {
        Self {
            field: "DATABASE_URL",
            message: format!(
                "DATABASE_URL is required when APP_ENV={} because production-like environments connect to an external PostgreSQL on boot",
                app_env.as_str()
            ),
        }
    }

    pub fn from_seed(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    pub const fn field(&self) -> &'static str {
        self.field
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigError {}

fn parse_bool(field: &'static str, raw: &str) -> Result<bool, ConfigError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        value => Err(ConfigError::invalid_bool(field, value)),
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, AppEnv, parse_bool};
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{Mutex, OnceLock},
    };

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn map_lookup(values: &[(&str, &str)]) -> HashMap<String, String> {
        values
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn defaults_match_bootstrap_document() {
        assert_eq!(
            AppConfig::default(),
            AppConfig {
                app_env: AppEnv::Dev,
                http_addr: "0.0.0.0:8080".to_owned(),
                database_url: "postgres://127.0.0.1:5432/postgres".to_owned(),
                init_db_on_boot: false,
                session_cookie_secure: false,
                init_admin_username: None,
                init_admin_password: None,
                init_users_file: None,
                static_dir: PathBuf::from("apps/web/dist"),
                openapi_export_path: PathBuf::from("docs/artifacts/openapi.json"),
            }
        );
    }

    #[test]
    fn from_lookup_uses_defaults_when_env_is_missing() {
        assert_eq!(
            AppConfig::from_lookup(|_| None).expect("config should load"),
            AppConfig::default()
        );
    }

    #[test]
    fn from_lookup_reads_all_supported_overrides() {
        let values = map_lookup(&[
            ("APP_ENV", "dev"),
            ("HTTP_ADDR", "127.0.0.1:9090"),
            (
                "DATABASE_URL",
                "postgres://db.example/mini_conf_prod_candidate",
            ),
            ("INIT_DB_ON_BOOT", "true"),
            ("SESSION_COOKIE_SECURE", "true"),
            ("INIT_USERS_FILE", "config/bootstrap-users.yaml"),
            ("STATIC_DIR", "var/web"),
            ("OPENAPI_EXPORT_PATH", "var/openapi.json"),
        ]);

        let config =
            AppConfig::from_lookup(|key| values.get(key).cloned()).expect("config should load");

        assert_eq!(
            config,
            AppConfig {
                app_env: AppEnv::Dev,
                http_addr: "127.0.0.1:9090".to_owned(),
                database_url: "postgres://db.example/mini_conf_prod_candidate".to_owned(),
                init_db_on_boot: true,
                session_cookie_secure: true,
                init_admin_username: None,
                init_admin_password: None,
                init_users_file: Some(PathBuf::from("config/bootstrap-users.yaml")),
                static_dir: PathBuf::from("var/web"),
                openapi_export_path: PathBuf::from("var/openapi.json"),
            }
        );
    }

    #[test]
    fn from_lookup_accepts_common_app_env_aliases() {
        for (raw, expected) in [
            ("dev", AppEnv::Dev),
            ("development", AppEnv::Dev),
            ("test", AppEnv::Test),
            ("testing", AppEnv::Test),
            ("staging", AppEnv::Staging),
            ("prod", AppEnv::Prod),
            ("production", AppEnv::Prod),
        ] {
            let config = AppConfig::from_lookup(|key| match key {
                "APP_ENV" => Some(raw.to_owned()),
                "DATABASE_URL" if matches!(expected, AppEnv::Staging | AppEnv::Prod) => {
                    Some("postgres://db.example/mini_conf".to_owned())
                }
                _ => None,
            })
            .expect("config should load");

            assert_eq!(config.app_env, expected, "APP_ENV={raw} should parse");
        }
    }

    #[test]
    fn app_env_parse_accepts_supported_values_directly() {
        for (raw, expected) in [
            (" dev ", AppEnv::Dev),
            ("DEVELOPMENT", AppEnv::Dev),
            ("test", AppEnv::Test),
            ("Testing", AppEnv::Test),
            ("staging", AppEnv::Staging),
            ("PROD", AppEnv::Prod),
            ("production", AppEnv::Prod),
        ] {
            assert_eq!(
                AppEnv::parse(raw),
                Ok(expected),
                "APP_ENV={raw} should parse directly"
            );
        }
    }

    #[test]
    fn app_env_parse_rejects_unknown_values_directly() {
        let error = AppEnv::parse("qa").expect_err("unknown APP_ENV should be rejected");

        assert_eq!(error.field(), "APP_ENV");
        assert_eq!(error.to_string(), "APP_ENV: unsupported APP_ENV value: qa");
    }

    #[test]
    fn from_lookup_rejects_unknown_app_env() {
        let error = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("qa".to_owned()),
            _ => None,
        })
        .expect_err("config should reject unknown app env");

        assert_eq!(error.field(), "APP_ENV");
        assert_eq!(error.to_string(), "APP_ENV: unsupported APP_ENV value: qa");
    }

    #[test]
    fn from_lookup_parses_boolean_db_boot_flag() {
        for raw in ["1", "true", "yes", "on"] {
            let config = AppConfig::from_lookup(|key| match key {
                "INIT_DB_ON_BOOT" => Some(raw.to_owned()),
                _ => None,
            })
            .expect("config should load");

            assert!(
                config.init_db_on_boot,
                "INIT_DB_ON_BOOT={raw} should enable DB boot"
            );
        }
    }

    #[test]
    fn parse_bool_accepts_enabled_values_directly() {
        for raw in [" 1 ", "TRUE", "yes", "On"] {
            assert_eq!(
                parse_bool("TEST_BOOL", raw),
                Ok(true),
                "{raw} should parse as true"
            );
        }
    }

    #[test]
    fn from_lookup_parses_disabled_boolean_db_boot_flag() {
        for raw in ["0", "false", "no", "off"] {
            let config = AppConfig::from_lookup(|key| match key {
                "INIT_DB_ON_BOOT" => Some(raw.to_owned()),
                _ => None,
            })
            .expect("config should load");

            assert!(
                !config.init_db_on_boot,
                "INIT_DB_ON_BOOT={raw} should disable DB boot"
            );
        }
    }

    #[test]
    fn from_lookup_enables_secure_cookies_in_prod_by_default() {
        let config = AppConfig::from_lookup(|key| match key {
            "APP_ENV" => Some("prod".to_owned()),
            "DATABASE_URL" => Some("postgres://db.example/mini_conf".to_owned()),
            _ => None,
        })
        .expect("config should load");

        assert!(config.session_cookie_secure);
    }

    #[test]
    fn from_lookup_requires_database_url_in_production_like_envs() {
        for app_env in ["staging", "prod"] {
            let error = AppConfig::from_lookup(|key| match key {
                "APP_ENV" => Some(app_env.to_owned()),
                _ => None,
            })
            .expect_err("config should require database url in production-like envs");

            assert_eq!(error.field(), "DATABASE_URL");
            assert_eq!(
                error.to_string(),
                format!(
                    "DATABASE_URL: DATABASE_URL is required when APP_ENV={app_env} because production-like environments connect to an external PostgreSQL on boot"
                )
            );
        }
    }

    #[test]
    fn production_like_envs_connect_database_on_boot_without_db_boot() {
        for app_env in ["staging", "prod"] {
            let config = AppConfig::from_lookup(|key| match key {
                "APP_ENV" => Some(app_env.to_owned()),
                "DATABASE_URL" => Some("postgres://db.example/mini_conf".to_owned()),
                _ => None,
            })
            .expect("config should load");

            assert!(!config.init_db_on_boot);
            assert!(config.should_connect_database_on_boot());
        }
    }

    #[test]
    fn parse_bool_accepts_disabled_values_directly() {
        for raw in [" 0 ", "FALSE", "no", "Off"] {
            assert_eq!(
                parse_bool("TEST_BOOL", raw),
                Ok(false),
                "{raw} should parse as false"
            );
        }
    }

    #[test]
    fn parse_bool_rejects_unknown_values_directly() {
        let error =
            parse_bool("TEST_BOOL", "sometimes").expect_err("unknown bool should be rejected");

        assert_eq!(error.field(), "TEST_BOOL");
        assert_eq!(
            error.to_string(),
            "TEST_BOOL: unsupported TEST_BOOL value: sometimes"
        );
    }

    #[test]
    fn from_lookup_rejects_unknown_boolean_flag() {
        let error = AppConfig::from_lookup(|key| match key {
            "INIT_DB_ON_BOOT" => Some("sometimes".to_owned()),
            _ => None,
        })
        .expect_err("config should reject unknown boolean flag");

        assert_eq!(error.field(), "INIT_DB_ON_BOOT");
        assert_eq!(
            error.to_string(),
            "INIT_DB_ON_BOOT: unsupported INIT_DB_ON_BOOT value: sometimes"
        );
    }

    #[test]
    fn from_lookup_rejects_db_boot_outside_dev_and_test() {
        for app_env in ["staging", "prod"] {
            let error = AppConfig::from_lookup(|key| match key {
                "APP_ENV" => Some(app_env.to_owned()),
                "INIT_DB_ON_BOOT" => Some("true".to_owned()),
                _ => None,
            })
            .expect_err("config should reject db boot outside dev and test");

            assert_eq!(error.field(), "INIT_DB_ON_BOOT");
            assert_eq!(
                error.to_string(),
                format!(
                    "INIT_DB_ON_BOOT: INIT_DB_ON_BOOT=true is only supported for APP_ENV=dev or APP_ENV=test, got {app_env}"
                )
            );
        }
    }

    #[test]
    fn path_accessors_expose_configured_paths() {
        let config = AppConfig::from_lookup(|key| match key {
            "STATIC_DIR" => Some("runtime/static".to_owned()),
            "OPENAPI_EXPORT_PATH" => Some("runtime/openapi.json".to_owned()),
            _ => None,
        })
        .expect("config should load");

        assert_eq!(
            config.static_dir(),
            PathBuf::from("runtime/static").as_path()
        );
        assert_eq!(
            config.openapi_export_path(),
            PathBuf::from("runtime/openapi.json").as_path()
        );
    }

    #[test]
    fn from_env_reads_real_environment() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        // SAFETY: tests serialize access to process env with a mutex.
        unsafe {
            std::env::set_var("APP_ENV", "test");
            std::env::set_var("HTTP_ADDR", "127.0.0.1:7001");
            std::env::set_var("DATABASE_URL", "postgres://override/mini_conf_test");
            std::env::set_var("INIT_DB_ON_BOOT", "1");
            std::env::set_var("SESSION_COOKIE_SECURE", "1");
            std::env::set_var("STATIC_DIR", "tmp/static");
            std::env::set_var("OPENAPI_EXPORT_PATH", "tmp/openapi.json");
        }

        let config = AppConfig::from_env().expect("config should load");

        // SAFETY: tests serialize access to process env with a mutex.
        unsafe {
            std::env::remove_var("APP_ENV");
            std::env::remove_var("HTTP_ADDR");
            std::env::remove_var("DATABASE_URL");
            std::env::remove_var("INIT_DB_ON_BOOT");
            std::env::remove_var("SESSION_COOKIE_SECURE");
            std::env::remove_var("STATIC_DIR");
            std::env::remove_var("OPENAPI_EXPORT_PATH");
        }

        assert_eq!(
            config,
            AppConfig {
                app_env: AppEnv::Test,
                http_addr: "127.0.0.1:7001".to_owned(),
                database_url: "postgres://override/mini_conf_test".to_owned(),
                init_db_on_boot: true,
                session_cookie_secure: true,
                init_admin_username: None,
                init_admin_password: None,
                init_users_file: None,
                static_dir: PathBuf::from("tmp/static"),
                openapi_export_path: PathBuf::from("tmp/openapi.json"),
            }
        );
    }
}
