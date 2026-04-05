use infra::AppIdentity;
use server::{app, bootstrap, config::ServerConfig};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    bootstrap::init_tracing();

    let config = ServerConfig::from_env();
    let identity = AppIdentity::new("mini-conf-server", env!("CARGO_PKG_VERSION"));
    let listener = TcpListener::bind(&config.http_addr).await?;

    tracing::info!(address = %config.http_addr, "starting mini-conf server");

    axum::serve(listener, app(identity)).await
}
