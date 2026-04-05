#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub http_addr: String,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self::from_http_addr_env(std::env::var("HTTP_ADDR").ok())
    }

    pub fn from_http_addr_env(http_addr: Option<String>) -> Self {
        let http_addr = http_addr.unwrap_or_else(|| "0.0.0.0:8080".to_owned());

        Self { http_addr }
    }
}

#[cfg(test)]
mod tests {
    use super::ServerConfig;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn uses_default_http_addr_when_env_is_missing() {
        assert_eq!(
            ServerConfig::from_http_addr_env(None),
            ServerConfig {
                http_addr: "0.0.0.0:8080".to_owned(),
            }
        );
    }

    #[test]
    fn uses_http_addr_from_explicit_value() {
        assert_eq!(
            ServerConfig::from_http_addr_env(Some("127.0.0.1:9090".to_owned())),
            ServerConfig {
                http_addr: "127.0.0.1:9090".to_owned(),
            }
        );
    }

    #[test]
    fn from_env_reads_http_addr_override() {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");

        // SAFETY: tests serialize access to process env with a mutex.
        unsafe {
            std::env::set_var("HTTP_ADDR", "127.0.0.1:7001");
        }

        let config = ServerConfig::from_env();

        // SAFETY: tests serialize access to process env with a mutex.
        unsafe {
            std::env::remove_var("HTTP_ADDR");
        }

        assert_eq!(
            config,
            ServerConfig {
                http_addr: "127.0.0.1:7001".to_owned(),
            }
        );
    }
}
