//! demo-coffee-seed — Seeds the coffee demo into an isolated PostgreSQL schema.
//!
//! Reads `DATABASE_URL` (must already point at the demo schema via search_path).
//! Writes `demo/coffee/generated/current-run.json` with runtime connection info.
//!
//! Intended to be called by `scripts/demo-coffee-reset.sh`, which handles schema
//! creation and migration before invoking this binary.

use server::auth::{hash_bearer_token, hash_password};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::{env, error::Error, io, path::PathBuf};

const SEED_TAG: &str = "coffee-demo-v1";

const ADMIN_USERNAME: &str = "admin";
const ADMIN_PASSWORD: &str = "admin123456";

const PROJECT_CODE: &str = "coffee-middleware-demo";
const PROJECT_NAME: &str = "Coffee Middleware Demo";
const PROJECT_DESCRIPTION: &str = "End-to-end demo: template cloning, multi-backend SN routing, publish → pull → apply → heartbeat.";

// Credential tokens — plaintext values used by demo-coffee-client and demo-coffee-backend.
const TOKEN_A_PROD_001: &str = "mc_demo_coffee_a_prod_001";
const TOKEN_A_PROD_002: &str = "mc_demo_coffee_a_prod_002";
const TOKEN_B_PROD_001: &str = "mc_demo_coffee_b_prod_001";

type SeedResult<T = ()> = Result<T, Box<dyn Error>>;

// ---------------------------------------------------------------------------
// Seed parameter structs
// ---------------------------------------------------------------------------

struct ConfigSeed<'a> {
    project_id: i64,
    code: &'a str,
    name: &'a str,
    format: &'a str,
    sensitivity: &'a str,
    secret_paths: Option<serde_json::Value>,
    description: Option<&'a str>,
    is_required: bool,
    status: &'a str,
}

struct ProjectEnvironmentSeed<'a> {
    project_id: i64,
    code: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    status: &'a str,
    sort_order: i32,
}

struct DeploymentSeed<'a> {
    project_id: i64,
    environment_id: i64,
    deployment_key: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    is_template: bool,
    template_source_id: Option<i64>,
    status: &'a str,
}

struct ReleaseSeed<'a> {
    project_id: i64,
    config_file_id: i64,
    deployment_instance_id: i64,
    revision: &'a str,
    content: &'a str,
    format: &'a str,
    change_summary: Option<&'a str>,
    diff_summary: serde_json::Value,
    apply_mode: &'a str,
    published_by: i64,
}

struct DraftSeed<'a> {
    project_id: i64,
    config_file_id: i64,
    deployment_instance_id: i64,
    content: &'a str,
    format: &'a str,
    version: i64,
    editor_user_id: i64,
}

struct SyncRecordSeed<'a> {
    project_id: i64,
    deployment_instance_id: i64,
    config_file_id: i64,
    release_id: Option<i64>,
    revision: Option<&'a str>,
    action: &'a str,
    status: &'a str,
    message: Option<&'a str>,
    detail: serde_json::Value,
    reported_at: &'a str,
}

struct HeartbeatSeed<'a> {
    project_id: i64,
    deployment_instance_id: i64,
    config_file_id: i64,
    metadata: serde_json::Value,
    reported_at: &'a str,
}

struct AuditSeed<'a> {
    project_id: Option<i64>,
    user_id: Option<i64>,
    action: &'a str,
    resource_type: &'a str,
    resource_id: String,
    detail: Option<serde_json::Value>,
    created_at: &'a str,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> SeedResult {
    let database_url = env::var("DATABASE_URL").map_err(|_| {
        io::Error::other(
            "DATABASE_URL is required; run through just demo-coffee-reset or export DATABASE_URL pointing at the demo schema",
        )
    })?;

    let config_center_url =
        env::var("CONFIG_CENTER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let summary = seed_demo_data(&pool, &config_center_url).await?;

    // Write generated/current-run.json relative to the repo root.
    write_current_run_json(&database_url, &config_center_url, &summary).await?;

    println!("=== Coffee Demo Seeded ===");
    println!("Schema:   {}", summary.schema);
    println!("Project:  {PROJECT_CODE}");
    println!("Login:    {ADMIN_USERNAME} / {ADMIN_PASSWORD}");
    println!();
    println!("Instances:");
    println!("  a-prod-store-001  (active)   token: {TOKEN_A_PROD_001}");
    println!("  a-prod-store-002  (inactive)  token: {TOKEN_A_PROD_002}");
    println!("  b-prod-store-001  (active)   token: {TOKEN_B_PROD_001}");
    println!();
    println!("SN routing:");
    println!("  backend-a:19001 / SN001 -> a-prod-store-001");
    println!("  backend-a:19001 / SN002 -> a-prod-store-002");
    println!("  backend-b:19002 / SN001 -> b-prod-store-001");
    println!();
    println!("Generated: demo/coffee/generated/current-run.json");

    Ok(())
}

// ---------------------------------------------------------------------------
// Summary returned by seed_demo_data
// ---------------------------------------------------------------------------

struct DemoSummary {
    schema: String,
    project_id: i64,
    project_code: &'static str,
    a_prod_001_id: i64,
}

// ---------------------------------------------------------------------------
// Core seed logic
// ---------------------------------------------------------------------------

async fn seed_demo_data(pool: &PgPool, _config_center_url: &str) -> SeedResult<DemoSummary> {
    let mut tx = pool.begin().await?;

    // Detect schema name from search_path for the summary output.
    let schema: String = sqlx::query_scalar("SELECT current_schema()")
        .fetch_one(tx.as_mut())
        .await?;

    // -----------------------------------------------------------------------
    // Users
    // -----------------------------------------------------------------------
    let admin_id = upsert_user(&mut tx, ADMIN_USERNAME, ADMIN_PASSWORD, "active").await?;

    // -----------------------------------------------------------------------
    // Project
    // -----------------------------------------------------------------------
    let project_id = upsert_project(
        &mut tx,
        PROJECT_CODE,
        PROJECT_NAME,
        Some(PROJECT_DESCRIPTION),
        "active",
    )
    .await?;

    upsert_project_member(&mut tx, project_id, admin_id, "admin").await?;

    // -----------------------------------------------------------------------
    // Environments
    // -----------------------------------------------------------------------
    let _dev_env_id = upsert_project_environment(
        &mut tx,
        ProjectEnvironmentSeed {
            project_id,
            code: "dev",
            name: "Development",
            description: Some("Development environment."),
            status: "active",
            sort_order: 10,
        },
    )
    .await?;

    let prod_env_id = upsert_project_environment(
        &mut tx,
        ProjectEnvironmentSeed {
            project_id,
            code: "prod",
            name: "Production",
            description: Some("Production environment."),
            status: "active",
            sort_order: 20,
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Config files
    // -----------------------------------------------------------------------
    let coffee_main_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id,
            code: "coffee-main",
            name: "Coffee Main",
            format: "toml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Primary store runtime settings."),
            is_required: true,
            status: "active",
        },
    )
    .await?;

    let store_flags_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id,
            code: "store-flags",
            name: "Store Feature Flags",
            format: "toml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Optional per-store feature toggles."),
            is_required: false,
            status: "active",
        },
    )
    .await?;

    let _store_secret_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id,
            code: "store-secret",
            name: "Store Secret",
            format: "toml",
            sensitivity: "secret",
            secret_paths: Some(serde_json::json!([
                "$.credentials.api_key",
                "$.credentials.webhook_secret"
            ])),
            description: Some("Secret store credentials; redacted on management reads."),
            is_required: false,
            status: "active",
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Template instance
    // -----------------------------------------------------------------------
    let template_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id,
            environment_id: prod_env_id,
            deployment_key: "tpl-coffee-store-basic",
            name: "Template Coffee Store Basic",
            description: Some("Canonical template for cloning new store deployments."),
            is_template: true,
            template_source_id: None,
            status: "inactive",
        },
    )
    .await?;

    // Seed template drafts so clone gets pre-populated content.
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id,
            config_file_id: coffee_main_id,
            deployment_instance_id: template_id,
            content: "[runtime]\nlog_level = \"info\"\npoll_interval_seconds = 30\n\n[feature]\nenable_hot_reload = false\nrecommendation_banner = \"\"\n",
            format: "toml",
            version: 1,
            editor_user_id: admin_id,
        },
    )
    .await?;

    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id,
            config_file_id: store_flags_id,
            deployment_instance_id: template_id,
            content: "[flags]\nloyalty_program = true\ndigital_menu = false\n",
            format: "toml",
            version: 1,
            editor_user_id: admin_id,
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Deployment instances
    // -----------------------------------------------------------------------
    let a_prod_001_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id,
            environment_id: prod_env_id,
            deployment_key: "a-prod-store-001",
            name: "Backend-A Prod Store 001",
            description: Some("Primary active instance; exercises the happy-path lifecycle."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "active",
        },
    )
    .await?;

    let a_prod_002_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id,
            environment_id: prod_env_id,
            deployment_key: "a-prod-store-002",
            name: "Backend-A Prod Store 002",
            description: Some("Inactive instance; demonstrates lifecycle gating on open API."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "inactive",
        },
    )
    .await?;

    let b_prod_001_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id,
            environment_id: prod_env_id,
            deployment_key: "b-prod-store-001",
            name: "Backend-B Prod Store 001",
            description: Some("Different backend; shows SN001 can repeat across backends."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "active",
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Credentials
    // -----------------------------------------------------------------------
    upsert_credential(&mut tx, a_prod_001_id, "default", TOKEN_A_PROD_001).await?;
    upsert_credential(&mut tx, a_prod_002_id, "default", TOKEN_A_PROD_002).await?;
    upsert_credential(&mut tx, b_prod_001_id, "default", TOKEN_B_PROD_001).await?;

    // -----------------------------------------------------------------------
    // Releases — coffee-main for a-prod-store-001 (two revisions for diff demo)
    // -----------------------------------------------------------------------
    let a001_main_rev1_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id,
            config_file_id: coffee_main_id,
            deployment_instance_id: a_prod_001_id,
            revision: "20260421.0001",
            content: "[runtime]\nlog_level = \"info\"\npoll_interval_seconds = 30\n\n[feature]\nenable_hot_reload = false\nrecommendation_banner = \"\"\n",
            format: "toml",
            change_summary: Some("Initial coffee-main release for a-prod-store-001"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 7,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_id,
        },
    )
    .await?;

    // store-flags release for a-prod-store-001
    let a001_flags_rev_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id,
            config_file_id: store_flags_id,
            deployment_instance_id: a_prod_001_id,
            revision: "20260421.0002",
            content: "[flags]\nloyalty_program = true\ndigital_menu = false\n",
            format: "toml",
            change_summary: Some("Initial store-flags for a-prod-store-001"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 3,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_id,
        },
    )
    .await?;

    // coffee-main release for b-prod-store-001
    let b001_main_rev_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id,
            config_file_id: coffee_main_id,
            deployment_instance_id: b_prod_001_id,
            revision: "20260421.0003",
            content: "[runtime]\nlog_level = \"warn\"\npoll_interval_seconds = 60\n\n[feature]\nenable_hot_reload = false\nrecommendation_banner = \"\"\n",
            format: "toml",
            change_summary: Some("Initial coffee-main for b-prod-store-001"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 7,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_id,
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Draft for a-prod-store-001 / coffee-main — ready for the demo "publish" step
    // -----------------------------------------------------------------------
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id,
            config_file_id: coffee_main_id,
            deployment_instance_id: a_prod_001_id,
            content: "[runtime]\nlog_level = \"info\"\npoll_interval_seconds = 10\n\n[feature]\nenable_hot_reload = true\nrecommendation_banner = \"spring latte\"\n",
            format: "toml",
            version: 2,
            editor_user_id: admin_id,
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Sync records — simulated "client applied initial revision" state
    // -----------------------------------------------------------------------
    clear_seeded_sync_records(&mut tx).await?;

    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id,
            deployment_instance_id: a_prod_001_id,
            config_file_id: coffee_main_id,
            release_id: Some(a001_main_rev1_id),
            revision: Some("20260421.0001"),
            action: "apply",
            status: "success",
            message: Some("initial apply"),
            detail: serde_json::json!({"duration_ms": 54, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-21T09:00:00Z",
        },
    )
    .await?;

    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id,
            deployment_instance_id: a_prod_001_id,
            config_file_id: store_flags_id,
            release_id: Some(a001_flags_rev_id),
            revision: Some("20260421.0002"),
            action: "apply",
            status: "success",
            message: Some("flags applied"),
            detail: serde_json::json!({"duration_ms": 31, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-21T09:00:05Z",
        },
    )
    .await?;

    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id,
            deployment_instance_id: b_prod_001_id,
            config_file_id: coffee_main_id,
            release_id: Some(b001_main_rev_id),
            revision: Some("20260421.0003"),
            action: "apply",
            status: "success",
            message: Some("initial apply"),
            detail: serde_json::json!({"duration_ms": 48, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-21T09:01:00Z",
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Heartbeats — simulated "client is alive" state
    // -----------------------------------------------------------------------
    upsert_heartbeat(
        &mut tx,
        HeartbeatSeed {
            project_id,
            deployment_instance_id: a_prod_001_id,
            config_file_id: coffee_main_id,
            metadata: serde_json::json!({
                "status": "ready",
                "ip": "10.0.1.101",
                "version": "2.1.0",
                "applied_revision": "20260421.0001",
                "seed_tag": SEED_TAG
            }),
            reported_at: "2026-04-21T09:05:00Z",
        },
    )
    .await?;

    upsert_heartbeat(
        &mut tx,
        HeartbeatSeed {
            project_id,
            deployment_instance_id: b_prod_001_id,
            config_file_id: coffee_main_id,
            metadata: serde_json::json!({
                "status": "ready",
                "ip": "10.0.2.101",
                "version": "2.0.5",
                "applied_revision": "20260421.0003",
                "seed_tag": SEED_TAG
            }),
            reported_at: "2026-04-21T09:06:00Z",
        },
    )
    .await?;

    // -----------------------------------------------------------------------
    // Audit log
    // -----------------------------------------------------------------------
    clear_seeded_audit_logs(&mut tx).await?;

    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(project_id),
            user_id: Some(admin_id),
            action: "project.created",
            resource_type: "project",
            resource_id: project_id.to_string(),
            detail: Some(serde_json::json!({"seed_tag": SEED_TAG, "project_code": PROJECT_CODE})),
            created_at: "2026-04-21T08:00:00Z",
        },
    )
    .await?;

    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(project_id),
            user_id: Some(admin_id),
            action: "deployment_instance.cloned",
            resource_type: "deployment_instance",
            resource_id: a_prod_001_id.to_string(),
            detail: Some(serde_json::json!({
                "seed_tag": SEED_TAG,
                "source_kind": "template",
                "template_source_id": template_id,
                "deployment_key": "a-prod-store-001"
            })),
            created_at: "2026-04-21T08:05:00Z",
        },
    )
    .await?;

    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(project_id),
            user_id: Some(admin_id),
            action: "release.published",
            resource_type: "release",
            resource_id: a001_main_rev1_id.to_string(),
            detail: Some(serde_json::json!({
                "seed_tag": SEED_TAG,
                "revision": "20260421.0001",
                "deployment_key": "a-prod-store-001",
                "config_code": "coffee-main"
            })),
            created_at: "2026-04-21T08:10:00Z",
        },
    )
    .await?;

    tx.commit().await?;

    Ok(DemoSummary {
        schema,
        project_id,
        project_code: PROJECT_CODE,
        a_prod_001_id,
    })
}

// ---------------------------------------------------------------------------
// Write generated/current-run.json
// ---------------------------------------------------------------------------

async fn write_current_run_json(
    database_url: &str,
    config_center_url: &str,
    summary: &DemoSummary,
) -> SeedResult {
    // Locate the repo root: navigate up from the binary's working directory.
    // When invoked via `cargo run` from the repo root, `env::current_dir()` is the repo root.
    let repo_root = env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap_or_default());

    // Cargo manifest dir is apps/server; go up two levels to repo root.
    let repo_root = if repo_root.ends_with("apps/server") {
        repo_root
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&repo_root)
            .to_path_buf()
    } else {
        repo_root
    };

    let generated_dir = repo_root.join("demo/coffee/generated");
    tokio::fs::create_dir_all(&generated_dir).await?;

    let current_run = serde_json::json!({
        "schema": summary.schema,
        "database_url": database_url,
        "config_center_url": config_center_url,
        "backend_a_url": "http://127.0.0.1:19001",
        "backend_b_url": "http://127.0.0.1:19002",
        "project_code": summary.project_code,
        "project_id": summary.project_id,
        "a_prod_001_id": summary.a_prod_001_id,
        "tokens": {
            "a_prod_store_001": TOKEN_A_PROD_001,
            "a_prod_store_002": TOKEN_A_PROD_002,
            "b_prod_store_001": TOKEN_B_PROD_001
        },
        "sn_routing": {
            "backend-a": {
                "SN001": "a-prod-store-001",
                "SN002": "a-prod-store-002"
            },
            "backend-b": {
                "SN001": "b-prod-store-001"
            }
        },
        "users": {
            "admin": ADMIN_USERNAME
        },
        "admin_username": ADMIN_USERNAME,
        "admin_password": ADMIN_PASSWORD
    });

    let json_path = generated_dir.join("current-run.json");
    tokio::fs::write(&json_path, serde_json::to_string_pretty(&current_run)?).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Database helpers (mirrors dev-seed-demo.rs helpers)
// ---------------------------------------------------------------------------

async fn upsert_user(
    tx: &mut Transaction<'_, Postgres>,
    username: &str,
    password: &str,
    status: &str,
) -> SeedResult<i64> {
    let password_hash =
        hash_password(password).map_err(|error| io::Error::other(error.into_body().message))?;

    let user_id = sqlx::query_scalar(
        r#"
        INSERT INTO users (username, password_hash, status)
        VALUES ($1, $2, $3)
        ON CONFLICT (username)
        DO UPDATE SET
            password_hash = EXCLUDED.password_hash,
            status = EXCLUDED.status,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .bind(status)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(user_id)
}

async fn upsert_project(
    tx: &mut Transaction<'_, Postgres>,
    code: &str,
    name: &str,
    description: Option<&str>,
    status: &str,
) -> SeedResult<i64> {
    let project_id = sqlx::query_scalar(
        r#"
        INSERT INTO projects (code, name, description, status)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (code)
        DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            status = EXCLUDED.status,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(code)
    .bind(name)
    .bind(description)
    .bind(status)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(project_id)
}

async fn upsert_project_member(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    user_id: i64,
    role: &str,
) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, user_id)
        DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .bind(role)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn upsert_project_environment(
    tx: &mut Transaction<'_, Postgres>,
    seed: ProjectEnvironmentSeed<'_>,
) -> SeedResult<i64> {
    let environment_id = sqlx::query_scalar(
        r#"
        INSERT INTO project_environments (
            project_id, code, name, description, status, sort_order
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (project_id, code)
        DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            status = EXCLUDED.status,
            sort_order = EXCLUDED.sort_order,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.code)
    .bind(seed.name)
    .bind(seed.description)
    .bind(seed.status)
    .bind(seed.sort_order)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(environment_id)
}

async fn upsert_config_file(
    tx: &mut Transaction<'_, Postgres>,
    seed: ConfigSeed<'_>,
) -> SeedResult<i64> {
    let config_file_id = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id, code, name, format, sensitivity,
            secret_paths, description, is_required, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (project_id, code)
        DO UPDATE SET
            name = EXCLUDED.name,
            format = EXCLUDED.format,
            sensitivity = EXCLUDED.sensitivity,
            secret_paths = EXCLUDED.secret_paths,
            description = EXCLUDED.description,
            is_required = EXCLUDED.is_required,
            status = EXCLUDED.status,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.code)
    .bind(seed.name)
    .bind(seed.format)
    .bind(seed.sensitivity)
    .bind(seed.secret_paths)
    .bind(seed.description)
    .bind(seed.is_required)
    .bind(seed.status)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(config_file_id)
}

async fn upsert_deployment_instance(
    tx: &mut Transaction<'_, Postgres>,
    seed: DeploymentSeed<'_>,
) -> SeedResult<i64> {
    let deployment_id = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id, environment_id, deployment_key,
            name, description, is_template, template_source_id, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (project_id, environment_id, deployment_key)
        WHERE deleted_at IS NULL
        DO UPDATE SET
            name = EXCLUDED.name,
            description = EXCLUDED.description,
            is_template = EXCLUDED.is_template,
            template_source_id = EXCLUDED.template_source_id,
            status = EXCLUDED.status,
            is_archived = FALSE,
            archived_at = NULL,
            archived_by = NULL,
            archive_reason = NULL,
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.environment_id)
    .bind(seed.deployment_key)
    .bind(seed.name)
    .bind(seed.description)
    .bind(seed.is_template)
    .bind(seed.template_source_id)
    .bind(seed.status)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(deployment_id)
}

async fn upsert_credential(
    tx: &mut Transaction<'_, Postgres>,
    deployment_instance_id: i64,
    credential_name: &str,
    token: &str,
) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO deployment_credentials (
            deployment_instance_id, credential_name, token_hash, status, last_used_at
        )
        VALUES ($1, $2, $3, 'active', NULL)
        ON CONFLICT (deployment_instance_id, credential_name)
        DO UPDATE SET
            token_hash = EXCLUDED.token_hash,
            status = 'active',
            last_used_at = NULL,
            updated_at = NOW()
        "#,
    )
    .bind(deployment_instance_id)
    .bind(credential_name)
    .bind(hash_bearer_token(token))
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn upsert_release(
    tx: &mut Transaction<'_, Postgres>,
    seed: ReleaseSeed<'_>,
) -> SeedResult<i64> {
    let release_id = sqlx::query_scalar(
        r#"
        INSERT INTO releases (
            project_id, config_file_id, deployment_instance_id,
            revision, content, content_hash, format,
            change_summary, diff_summary, apply_mode, published_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (deployment_instance_id, config_file_id, revision)
        DO UPDATE SET
            content = EXCLUDED.content,
            content_hash = EXCLUDED.content_hash,
            format = EXCLUDED.format,
            change_summary = EXCLUDED.change_summary,
            diff_summary = EXCLUDED.diff_summary,
            apply_mode = EXCLUDED.apply_mode,
            published_by = EXCLUDED.published_by
        RETURNING id
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.config_file_id)
    .bind(seed.deployment_instance_id)
    .bind(seed.revision)
    .bind(seed.content)
    .bind(hash_bearer_token(seed.content))
    .bind(seed.format)
    .bind(seed.change_summary)
    .bind(seed.diff_summary)
    .bind(seed.apply_mode)
    .bind(seed.published_by)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(release_id)
}

async fn upsert_draft(tx: &mut Transaction<'_, Postgres>, seed: DraftSeed<'_>) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO drafts (
            project_id, config_file_id, deployment_instance_id,
            content, content_hash, format, version, editor_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (config_file_id, deployment_instance_id)
        DO UPDATE SET
            content = EXCLUDED.content,
            content_hash = EXCLUDED.content_hash,
            format = EXCLUDED.format,
            version = EXCLUDED.version,
            editor_user_id = EXCLUDED.editor_user_id,
            updated_at = NOW()
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.config_file_id)
    .bind(seed.deployment_instance_id)
    .bind(seed.content)
    .bind(hash_bearer_token(seed.content))
    .bind(seed.format)
    .bind(seed.version)
    .bind(seed.editor_user_id)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn clear_seeded_sync_records(tx: &mut Transaction<'_, Postgres>) -> SeedResult {
    sqlx::query("DELETE FROM deployment_sync_records WHERE detail ->> 'seed_tag' = $1")
        .bind(SEED_TAG)
        .execute(tx.as_mut())
        .await?;
    Ok(())
}

async fn insert_sync_record(
    tx: &mut Transaction<'_, Postgres>,
    seed: SyncRecordSeed<'_>,
) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO deployment_sync_records (
            project_id, deployment_instance_id, config_file_id,
            release_id, revision, action, status,
            message, detail, reported_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::timestamptz)
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.deployment_instance_id)
    .bind(seed.config_file_id)
    .bind(seed.release_id)
    .bind(seed.revision)
    .bind(seed.action)
    .bind(seed.status)
    .bind(seed.message)
    .bind(seed.detail)
    .bind(seed.reported_at)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn upsert_heartbeat(
    tx: &mut Transaction<'_, Postgres>,
    seed: HeartbeatSeed<'_>,
) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO deployment_heartbeats (
            project_id, deployment_instance_id, config_file_id,
            metadata, reported_at
        )
        VALUES ($1, $2, $3, $4, $5::timestamptz)
        ON CONFLICT (deployment_instance_id, config_file_id)
        DO UPDATE SET
            metadata = EXCLUDED.metadata,
            reported_at = EXCLUDED.reported_at,
            updated_at = NOW()
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.deployment_instance_id)
    .bind(seed.config_file_id)
    .bind(seed.metadata)
    .bind(seed.reported_at)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn clear_seeded_audit_logs(tx: &mut Transaction<'_, Postgres>) -> SeedResult {
    sqlx::query("DELETE FROM audit_logs WHERE detail ->> 'seed_tag' = $1")
        .bind(SEED_TAG)
        .execute(tx.as_mut())
        .await?;
    Ok(())
}

async fn insert_audit_log(tx: &mut Transaction<'_, Postgres>, seed: AuditSeed<'_>) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            project_id, user_id, action,
            resource_type, resource_id, detail, created_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::timestamptz)
        "#,
    )
    .bind(seed.project_id)
    .bind(seed.user_id)
    .bind(seed.action)
    .bind(seed.resource_type)
    .bind(seed.resource_id)
    .bind(seed.detail)
    .bind(seed.created_at)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}
