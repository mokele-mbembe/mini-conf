use server::{bootstrap, config::AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::init_tracing();

    let config = AppConfig::from_env()?;

    bootstrap::run(config).await?;

    Ok(())
}
