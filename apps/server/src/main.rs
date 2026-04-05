use server::{bootstrap, config::ServerConfig};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    bootstrap::init_tracing();

    bootstrap::run(ServerConfig::from_env()).await
}
