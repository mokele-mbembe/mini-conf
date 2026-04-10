use infra::testing::{test_database_url, unique_schema_name, with_search_path};
use server::{bootstrap, config::AppConfig};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn temp_seed_file(name: &str, content: &str) -> TestResult<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("mini-conf-{name}-{unique}.yaml"));
    fs::write(&path, content)?;
    Ok(path)
}

async fn setup_database(test_name: &str) -> TestResult<Option<(String, String)>> {
    let Some(database_url) = test_database_url(test_name) else {
        return Ok(None);
    };
    let schema = unique_schema_name("mini_conf_bootstrap_users");
    let mut admin = PgConnection::connect(&database_url).await?;
    admin
        .execute(format!("CREATE SCHEMA {schema}").as_str())
        .await?;

    Ok(Some((database_url, schema)))
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
async fn build_state_seeds_users_from_init_users_file_idempotently() -> TestResult {
    let Some((database_url, schema)) = setup_database("bootstrap users").await? else {
        return Ok(());
    };
    let seed_file = temp_seed_file(
        "users",
        r#"
users:
  - username: alice
    password: alice123
    status: active
  - username: bob
    password: bob123
    status: disabled
"#,
    )?;

    let config = AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        init_users_file: Some(seed_file.clone()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    };

    let state = bootstrap::build_state(config.clone()).await?;
    let pool = state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;

    let users: Vec<(String, String)> =
        sqlx::query_as("SELECT username, status FROM users ORDER BY username ASC")
            .fetch_all(&pool)
            .await?;
    assert_eq!(
        users,
        vec![
            ("admin".to_owned(), "active".to_owned()),
            ("alice".to_owned(), "active".to_owned()),
            ("bob".to_owned(), "disabled".to_owned())
        ]
    );

    let second_state = bootstrap::build_state(config).await?;
    let second_pool = second_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after second bootstrap"))?;
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&second_pool)
        .await?;
    assert_eq!(user_count, 3);

    second_pool.close().await;
    fs::remove_file(seed_file)?;
    teardown(&database_url, &schema, pool).await
}

#[tokio::test]
async fn build_state_can_bind_seeded_memberships_to_existing_projects() -> TestResult {
    let Some((database_url, schema)) = setup_database("bootstrap user memberships").await? else {
        return Ok(());
    };

    let initial_state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let pool = initial_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;
    sqlx::query("INSERT INTO projects (code, name, status) VALUES ('seed-project', 'Seed Project', 'active')")
        .execute(&pool)
        .await?;

    let seed_file = temp_seed_file(
        "memberships",
        r#"
users:
  - username: alice
    password: alice123
    memberships:
      - project_code: seed-project
        role: viewer
"#,
    )?;

    let seeded_state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        init_users_file: Some(seed_file.clone()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let seeded_pool = seeded_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after seeded bootstrap"))?;

    let role: String = sqlx::query_scalar(
        r#"
        SELECT pm.role
        FROM project_members pm
        JOIN users u ON u.id = pm.user_id
        JOIN projects p ON p.id = pm.project_id
        WHERE u.username = 'alice'
          AND p.code = 'seed-project'
        LIMIT 1
        "#,
    )
    .fetch_one(&seeded_pool)
    .await?;
    assert_eq!(role, "viewer");

    seeded_pool.close().await;
    fs::remove_file(seed_file)?;
    teardown(&database_url, &schema, pool).await
}
