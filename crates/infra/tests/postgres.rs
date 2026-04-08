use infra::testing::{test_database_url, unique_schema_name};
use sqlx::{
    Connection, Executor, PgConnection, migrate::Migrator, postgres::PgPoolOptions, query_scalar,
};
use std::{error::Error, path::PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

#[tokio::test]
async fn connect_establishes_pool_when_database_is_available() -> TestResult {
    let Some(database_url) = test_database_url("postgres") else {
        return Ok(());
    };

    let pool = infra::db::connect(&database_url).await?;

    let value: i32 = query_scalar("SELECT 1").fetch_one(&pool).await?;

    assert_eq!(value, 1);

    pool.close().await;

    Ok(())
}

#[tokio::test]
async fn migrations_run_up_and_down_in_an_isolated_schema() -> TestResult {
    let Some(database_url) = test_database_url("postgres") else {
        return Ok(());
    };

    let migrator = Migrator::new(migrations_dir().as_path()).await?;
    let schema = unique_schema_name("mini_conf_test");
    let mut connection = PgConnection::connect(&database_url).await?;

    connection
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;
    connection
        .execute(format!("SET search_path TO {schema}").as_str())
        .await?;

    migrator.run_direct(&mut connection).await?;
    migrator.run_direct(&mut connection).await?;

    let exists_after_up: bool = query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'bootstrap_metadata'
        )",
    )
    .fetch_one(&mut connection)
    .await?;

    assert!(
        exists_after_up,
        "bootstrap_metadata should exist after migrate up"
    );

    migrator.undo(&mut connection, 0).await?;

    let exists_after_down: bool = query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'bootstrap_metadata'
        )",
    )
    .fetch_one(&mut connection)
    .await?;

    assert!(
        !exists_after_down,
        "bootstrap_metadata should be removed after migrate down"
    );

    connection.execute("SET search_path TO public").await?;
    connection
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;

    Ok(())
}

#[tokio::test]
async fn migration_schema_can_be_used_through_a_pool_connection() -> TestResult {
    let Some(database_url) = test_database_url("postgres") else {
        return Ok(());
    };

    let schema = unique_schema_name("mini_conf_test");
    let mut admin = PgConnection::connect(&database_url).await?;

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;
    admin.execute("SET search_path TO public").await?;

    let database_url_with_schema = database_url
        .parse::<sqlx::postgres::PgConnectOptions>()
        .map_err(|error| -> Box<dyn Error> { Box::new(error) })?
        .options([("search_path", schema.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database_url_with_schema)
        .await?;

    let schema_name: String = query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await?;

    assert_eq!(schema_name, schema);

    pool.close().await;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await?;

    Ok(())
}
