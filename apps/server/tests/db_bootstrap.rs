use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use server::{bootstrap, config::AppConfig};
use sqlx::{Connection, Executor, PgConnection, PgPool, query_scalar};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn setup_pool() -> TestResult<Option<(PgPool, String, String)>> {
    let Some(database_url) = test_database_url("db bootstrap") else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_db_bootstrap");
    let mut admin = PgConnection::connect(&database_url).await?;

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;

    let pool = state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;

    Ok(Some((pool, database_url, schema)))
}

async fn teardown(database_url: &str, schema: &str, pool: PgPool) -> TestResult {
    pool.close().await;

    let mut admin = PgConnection::connect(database_url).await?;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;

    Ok(())
}

#[tokio::test]
async fn build_state_connects_when_db_boot_is_enabled_and_database_is_available() -> TestResult {
    let Some((pool, database_url, schema)) = setup_pool().await? else {
        return Ok(());
    };

    let value: i32 = query_scalar("SELECT 1").fetch_one(&pool).await?;

    assert_eq!(value, 1);

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}

#[tokio::test]
async fn build_state_applies_migrations_when_db_boot_is_enabled() -> TestResult {
    let Some((pool, database_url, schema)) = setup_pool().await? else {
        return Ok(());
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
    .await?;

    assert!(
        bootstrap_table_exists,
        "bootstrap_metadata should exist after bootstrap applies migrations"
    );

    teardown(&database_url, &schema, pool).await?;

    Ok(())
}
