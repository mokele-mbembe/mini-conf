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
async fn init_users_file_seed_does_not_overwrite_existing_user_state() -> TestResult {
    let Some((database_url, schema)) = setup_database("bootstrap users no overwrite").await? else {
        return Ok(());
    };

    let seed_file = temp_seed_file(
        "users-no-overwrite",
        r#"
users:
  - username: alice
    password: alice123
    status: active
    is_platform_admin: false
    must_change_password: false
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

    let replacement_hash = server::auth::hash_password("replacement123")
        .map_err(|error| std::io::Error::other(error.into_body().message))?;
    sqlx::query(
        r#"
        UPDATE users
        SET
            password_hash = $1,
            status = 'disabled',
            is_platform_admin = TRUE,
            must_change_password = TRUE,
            password_updated_at = NOW(),
            updated_at = NOW()
        WHERE username = 'alice'
        "#,
    )
    .bind(&replacement_hash)
    .execute(&pool)
    .await?;

    let second_state = bootstrap::build_state(config).await?;
    let second_pool = second_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after second bootstrap"))?;

    let alice_row: (String, String, bool, bool) = sqlx::query_as(
        r#"
        SELECT password_hash, status, is_platform_admin, must_change_password
        FROM users
        WHERE username = 'alice'
        LIMIT 1
        "#,
    )
    .fetch_one(&second_pool)
    .await?;

    assert_eq!(alice_row.0, replacement_hash);
    assert_eq!(alice_row.1, "disabled");
    assert!(alice_row.2);
    assert!(alice_row.3);

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

#[tokio::test]
async fn bootstrap_admin_seed_only_applies_on_first_insert() -> TestResult {
    let Some((database_url, schema)) = setup_database("bootstrap admin first insert").await? else {
        return Ok(());
    };

    let first_state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let pool = first_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after bootstrap"))?;

    let replacement_hash = server::auth::hash_password("replacement123")
        .map_err(|error| std::io::Error::other(error.into_body().message))?;
    sqlx::query(
        r#"
        UPDATE users
        SET
            password_hash = $1,
            status = 'disabled',
            is_platform_admin = FALSE,
            must_change_password = TRUE,
            password_updated_at = NOW(),
            updated_at = NOW()
        WHERE username = 'admin'
        "#,
    )
    .bind(&replacement_hash)
    .execute(&pool)
    .await?;

    let second_state = bootstrap::build_state(AppConfig {
        init_db_on_boot: true,
        init_admin_username: Some("admin".to_owned()),
        init_admin_password: Some("admin123456".to_owned()),
        database_url: with_search_path(&database_url, &schema),
        ..AppConfig::default()
    })
    .await?;
    let second_pool = second_state
        .db_pool()
        .cloned()
        .ok_or_else(|| std::io::Error::other("db pool should be present after second bootstrap"))?;

    let admin_row: (String, String, bool, bool) = sqlx::query_as(
        r#"
        SELECT password_hash, status, is_platform_admin, must_change_password
        FROM users
        WHERE username = 'admin'
        LIMIT 1
        "#,
    )
    .fetch_one(&second_pool)
    .await?;

    assert_eq!(admin_row.0, replacement_hash);
    assert_eq!(admin_row.1, "disabled");
    assert!(!admin_row.2);
    assert!(admin_row.3);

    second_pool.close().await;
    teardown(&database_url, &schema, pool).await
}
