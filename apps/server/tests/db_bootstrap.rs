use server::{bootstrap, config::AppConfig};
use sqlx::{Connection, Executor, PgConnection, PgPool, query_scalar};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping db bootstrap integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_db_bootstrap_{nanos}")
}

fn with_search_path(database_url: &str, schema: &str) -> String {
    let separator = if database_url.contains('?') { '&' } else { '?' };
    format!("{database_url}{separator}options[search_path]={schema}")
}

async fn setup_pool() -> Option<(PgPool, String, String)> {
    let database_url = test_database_url()?;
    let schema = unique_schema_name();
    let mut admin = PgConnection::connect(&database_url)
        .await
        .expect("admin connection should succeed");

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("schema should be created");

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await
    .expect("state should connect to the test database");

    let pool = state
        .db_pool()
        .expect("db pool should be present after bootstrap")
        .clone();

    Some((pool, database_url, schema))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url)
        .await
        .expect("admin connection should succeed");
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await
        .expect("schema should be dropped");
}

#[tokio::test]
async fn build_state_connects_when_db_boot_is_enabled_and_database_is_available() {
    let Some((pool, database_url, schema)) = setup_pool().await else {
        return;
    };

    let value: i32 = query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("query should succeed");

    assert_eq!(value, 1);

    teardown(&database_url, &schema, pool).await;
}

#[tokio::test]
async fn build_state_applies_migrations_when_db_boot_is_enabled() {
    let Some((pool, database_url, schema)) = setup_pool().await else {
        return;
    };

    let bootstrap_table_exists: bool = query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'bootstrap_metadata'
        )",
    )
    .fetch_one(&pool)
    .await
    .expect("table existence query should succeed");

    assert!(
        bootstrap_table_exists,
        "bootstrap_metadata should exist after bootstrap applies migrations"
    );

    teardown(&database_url, &schema, pool).await;
}
