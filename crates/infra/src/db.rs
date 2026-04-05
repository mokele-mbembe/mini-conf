use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

pub const DEFAULT_MAX_CONNECTIONS: u32 = 5;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(3))
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .connect(database_url)
        .await
}
