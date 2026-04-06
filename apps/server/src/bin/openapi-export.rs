use server::{config::AppConfig, openapi};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env()?;
    openapi::export_to(config.openapi_export_path())?;
    println!(
        "exported OpenAPI to {}",
        config.openapi_export_path().display()
    );
    Ok(())
}
