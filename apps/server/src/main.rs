use server::{bootstrap, config::AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    bootstrap::init_tracing();

    let config = AppConfig::from_env()?;
    let state = bootstrap::build_state(config).await?;

    bootstrap::run(state).await?;

    Ok(())
}
