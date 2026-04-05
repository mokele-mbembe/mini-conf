use server::{bootstrap, config::AppConfig};
use sqlx::query_scalar;

fn test_database_url() -> Option<String> {
    match std::env::var("TEST_DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) => {
            eprintln!("skipping db bootstrap integration test: TEST_DATABASE_URL not set");
            None
        }
    }
}

#[tokio::test]
async fn build_state_connects_when_db_boot_is_enabled_and_database_is_available() {
    let Some(database_url) = test_database_url() else {
        return;
    };

    let state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        database_url,
        ..AppConfig::default()
    })
    .await
    .expect("state should connect to the test database");

    let value: i64 = query_scalar("SELECT 1")
        .fetch_one(state.db_pool().expect("db pool should be present"))
        .await
        .expect("query should succeed");

    assert_eq!(value, 1);
}
