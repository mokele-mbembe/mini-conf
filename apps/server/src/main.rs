use server::{bootstrap, config::AppConfig};

#[tokio::main]
async fn main() -> Result<(), bootstrap::StartupError> {
    bootstrap::init_tracing();

    let config = AppConfig::from_env().map_err(bootstrap::StartupError::from)?;
    let state = bootstrap::build_state(config).await?;

    bootstrap::run(state).await?;

    Ok(())
}
