use crate::{app, config::ServerConfig, state::AppState};
use infra::AppIdentity;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt};

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt().with_env_filter(filter).with_target(false).init();
}

pub async fn run(config: ServerConfig) -> std::io::Result<()> {
    let listener = TcpListener::bind(&config.http_addr).await?;
    let state = AppState::new(AppIdentity::new(
        "mini-conf-server",
        env!("CARGO_PKG_VERSION"),
    ));

    tracing::info!(address = %config.http_addr, "starting mini-conf server");

    axum::serve(listener, app(state)).await
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::config::ServerConfig;

    #[tokio::test]
    async fn run_returns_error_when_http_addr_is_invalid() {
        let result = run(ServerConfig {
            http_addr: "invalid-address".to_owned(),
        })
        .await;

        assert!(
            result.is_err(),
            "run should fail for an invalid bind address"
        );
    }
}
