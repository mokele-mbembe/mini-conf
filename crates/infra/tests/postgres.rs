use sqlx::{
    Connection, Executor, PgConnection, migrate::Migrator, postgres::PgPoolOptions, query_scalar,
};
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping postgres integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

fn unique_schema_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    format!("mini_conf_test_{nanos}")
}

#[tokio::test]
async fn connect_establishes_pool_when_database_is_available() {
    let Some(database_url) = test_database_url() else {
        return;
    };

    let pool = infra::db::connect(&database_url)
        .await
        .expect("pool should connect");

    let value: i32 = query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("query should succeed");

    assert_eq!(value, 1);

    pool.close().await;
}

#[tokio::test]
async fn migrations_run_up_and_down_in_an_isolated_schema() {
    let Some(database_url) = test_database_url() else {
        return;
    };

    let migrator = Migrator::new(migrations_dir().as_path())
        .await
        .expect("migrator should load migrations");
    let schema = unique_schema_name();
    let mut connection = PgConnection::connect(&database_url)
        .await
        .expect("connection should succeed");

    connection
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("schema should be created");
    connection
        .execute(format!("SET search_path TO {schema}").as_str())
        .await
        .expect("search_path should be set");

    migrator
        .run_direct(&mut connection)
        .await
        .expect("migrations should apply");
    migrator
        .run_direct(&mut connection)
        .await
        .expect("re-running migrations should be idempotent");

    let exists_after_up: bool = query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'bootstrap_metadata'
        )",
    )
    .fetch_one(&mut connection)
    .await
    .expect("table existence query should succeed");

    assert!(
        exists_after_up,
        "bootstrap_metadata should exist after migrate up"
    );

    migrator
        .undo(&mut connection, 0)
        .await
        .expect("migrations should roll back");

    let exists_after_down: bool = query_scalar(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'bootstrap_metadata'
        )",
    )
    .fetch_one(&mut connection)
    .await
    .expect("table existence query should succeed");

    assert!(
        !exists_after_down,
        "bootstrap_metadata should be removed after migrate down"
    );

    connection
        .execute("SET search_path TO public")
        .await
        .expect("search_path should reset");
    connection
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await
        .expect("schema should be dropped");
}

#[tokio::test]
async fn migration_schema_can_be_used_through_a_pool_connection() {
    let Some(database_url) = test_database_url() else {
        return;
    };

    let schema = unique_schema_name();
    let mut admin = PgConnection::connect(&database_url)
        .await
        .expect("connection should succeed");

    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await
        .expect("schema should be created");
    admin
        .execute("SET search_path TO public")
        .await
        .expect("search_path should reset");

    let database_url_with_schema = database_url
        .parse::<sqlx::postgres::PgConnectOptions>()
        .expect("database url should parse")
        .options([("search_path", schema.as_str())]);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database_url_with_schema)
        .await
        .expect("pool should connect with schema search_path");

    let schema_name: String = query_scalar("SELECT current_schema()")
        .fetch_one(&pool)
        .await
        .expect("query should succeed");

    assert_eq!(schema_name, schema);

    pool.close().await;
    admin
        .execute(format!("DROP SCHEMA IF EXISTS {schema} CASCADE").as_str())
        .await
        .expect("schema should be dropped");
}
