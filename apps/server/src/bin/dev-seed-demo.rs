use server::auth::{hash_bearer_token, hash_password};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};
use std::{env, error::Error, io};

const SEED_TAG: &str = "frontend-demo-v1";

const ADMIN_USERNAME: &str = "admin";
const ADMIN_PASSWORD: &str = "admin123456";
const ALICE_USERNAME: &str = "alice";
const ALICE_PASSWORD: &str = "alice123";
const BOB_USERNAME: &str = "bob";
const BOB_PASSWORD: &str = "bob123";
const CAROL_USERNAME: &str = "carol";
const CAROL_PASSWORD: &str = "carol123";

const STORE_001_TOKEN: &str = "mc_live_demo_store_001";
const STORE_002_TOKEN: &str = "mc_live_demo_store_002";
const STAGING_TOKEN: &str = "mc_live_demo_stage_001";

type SeedResult<T = ()> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
struct SeedSummary {
    coffee_project_code: &'static str,
    billing_project_code: &'static str,
    store_001_token: &'static str,
    store_002_token: &'static str,
    staging_token: &'static str,
}

#[tokio::main]
async fn main() -> SeedResult {
    let database_url = env::var("DATABASE_URL").map_err(|_| {
        io::Error::other(
            "DATABASE_URL is required; run through just dev-seed-demo-local or export DATABASE_URL",
        )
    })?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let summary = seed_demo_data(&pool).await?;

    println!("Seeded frontend demo data into runtime database.");
    println!("Login accounts:");
    println!("  - admin / {ADMIN_PASSWORD}");
    println!("  - alice / {ALICE_PASSWORD}");
    println!("  - bob / {BOB_PASSWORD}");
    println!("  - carol / {CAROL_PASSWORD}");
    println!("Projects:");
    println!("  - {}", summary.coffee_project_code);
    println!("  - {}", summary.billing_project_code);
    println!("Open API demo tokens:");
    println!("  - prod store-001: {}", summary.store_001_token);
    println!("  - prod store-002: {}", summary.store_002_token);
    println!("  - staging stage-001: {}", summary.staging_token);
    println!("Setup state:");
    println!("  - completed for local frontend preview");

    Ok(())
}

async fn seed_demo_data(pool: &PgPool) -> SeedResult<SeedSummary> {
    let mut tx = pool.begin().await?;

    let admin_user_id =
        upsert_user(&mut tx, ADMIN_USERNAME, ADMIN_PASSWORD, "active", true).await?;
    let alice_user_id =
        upsert_user(&mut tx, ALICE_USERNAME, ALICE_PASSWORD, "active", false).await?;
    let bob_user_id = upsert_user(&mut tx, BOB_USERNAME, BOB_PASSWORD, "active", false).await?;
    let carol_user_id =
        upsert_user(&mut tx, CAROL_USERNAME, CAROL_PASSWORD, "active", false).await?;

    let coffee_project_id = upsert_project(
        &mut tx,
        "demo-coffee-legacy",
        "Demo Coffee Legacy",
        Some("Frontend local preview project for deployment-first config workflows."),
        "active",
    )
    .await?;
    let billing_project_id = upsert_project(
        &mut tx,
        "demo-billing-service",
        "Demo Billing Service",
        Some("Smaller secondary project for project list and permission switching."),
        "active",
    )
    .await?;

    upsert_project_member(&mut tx, coffee_project_id, admin_user_id, "admin").await?;
    upsert_project_member(&mut tx, coffee_project_id, alice_user_id, "editor").await?;
    upsert_project_member(&mut tx, coffee_project_id, bob_user_id, "viewer").await?;
    upsert_project_member(&mut tx, billing_project_id, admin_user_id, "admin").await?;
    upsert_project_member(&mut tx, billing_project_id, alice_user_id, "viewer").await?;
    upsert_project_member(&mut tx, billing_project_id, carol_user_id, "admin").await?;

    let coffee_prod_environment_id = upsert_project_environment(
        &mut tx,
        ProjectEnvironmentSeed {
            project_id: coffee_project_id,
            code: "prod",
            name: "Production",
            description: Some("Primary production environment."),
            status: "active",
            sort_order: 10,
        },
    )
    .await?;
    let coffee_staging_environment_id = upsert_project_environment(
        &mut tx,
        ProjectEnvironmentSeed {
            project_id: coffee_project_id,
            code: "staging",
            name: "Staging",
            description: Some("Pre-release validation environment."),
            status: "active",
            sort_order: 20,
        },
    )
    .await?;
    let billing_prod_environment_id = upsert_project_environment(
        &mut tx,
        ProjectEnvironmentSeed {
            project_id: billing_project_id,
            code: "prod",
            name: "Production",
            description: Some("Primary production environment."),
            status: "active",
            sort_order: 10,
        },
    )
    .await?;

    let main_config_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id: coffee_project_id,
            code: "main",
            name: "Main Config",
            format: "yaml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Primary device runtime settings."),
            is_required: true,
            status: "active",
        },
    )
    .await?;
    let device_auth_config_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id: coffee_project_id,
            code: "device-auth",
            name: "Device Auth",
            format: "yaml",
            sensitivity: "secret",
            secret_paths: Some(serde_json::json!(["$.wifi.password", "$.cloud.api_key"])),
            description: Some("Secret device credentials; management reads should be redacted."),
            is_required: true,
            status: "active",
        },
    )
    .await?;
    let vision_config_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id: coffee_project_id,
            code: "vision",
            name: "Vision",
            format: "toml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Optional vision worker settings in TOML."),
            is_required: false,
            status: "active",
        },
    )
    .await?;
    let _ad_screen_config_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id: coffee_project_id,
            code: "ad-screen",
            name: "Ad Screen",
            format: "yaml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Archived config to exercise status filters."),
            is_required: false,
            status: "archived",
        },
    )
    .await?;
    let billing_main_config_id = upsert_config_file(
        &mut tx,
        ConfigSeed {
            project_id: billing_project_id,
            code: "main",
            name: "Main Config",
            format: "yaml",
            sensitivity: "normal",
            secret_paths: None,
            description: Some("Secondary project config for project switch testing."),
            is_required: true,
            status: "active",
        },
    )
    .await?;

    let template_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: coffee_project_id,
            environment_id: coffee_prod_environment_id,
            deployment_key: "template-default-store",
            name: "Template Default Store",
            description: Some("Template instance used for clone flows."),
            is_template: true,
            template_source_id: None,
            status: "inactive",
        },
    )
    .await?;
    let store_001_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: coffee_project_id,
            environment_id: coffee_prod_environment_id,
            deployment_key: "store-001",
            name: "Store 001",
            description: Some(
                "Main happy-path deployment with releases, drafts, heartbeats, and sync records.",
            ),
            is_template: false,
            template_source_id: Some(template_id),
            status: "active",
        },
    )
    .await?;
    let store_002_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: coffee_project_id,
            environment_id: coffee_prod_environment_id,
            deployment_key: "store-002",
            name: "Store 002",
            description: Some("Deployment intentionally missing one required config to exercise preview and publish blocking."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "active",
        },
    )
    .await?;
    let staging_store_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: coffee_project_id,
            environment_id: coffee_staging_environment_id,
            deployment_key: "stage-001",
            name: "Stage 001",
            description: Some("Secondary environment for list filter checks."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "active",
        },
    )
    .await?;
    let inactive_store_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: coffee_project_id,
            environment_id: coffee_prod_environment_id,
            deployment_key: "store-inactive",
            name: "Store Inactive",
            description: Some("Inactive deployment for status filter checks."),
            is_template: false,
            template_source_id: Some(template_id),
            status: "inactive",
        },
    )
    .await?;
    let billing_store_id = upsert_deployment_instance(
        &mut tx,
        DeploymentSeed {
            project_id: billing_project_id,
            environment_id: billing_prod_environment_id,
            deployment_key: "billing-001",
            name: "Billing 001",
            description: Some("Secondary project deployment."),
            is_template: false,
            template_source_id: None,
            status: "active",
        },
    )
    .await?;

    upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: store_001_id,
            revision: "20260410.0101",
            content: "log_level: info\npoll_interval_ms: 3000\n",
            format: "yaml",
            change_summary: Some("Initial store-001 main release"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_user_id,
        },
    )
    .await?;
    let store_001_main_release_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: store_001_id,
            revision: "20260412.0102",
            content: "log_level: warn\npoll_interval_ms: 5000\n",
            format: "yaml",
            change_summary: Some("Raise polling interval for store-001"),
            diff_summary: serde_json::json!({
                "is_initial": false,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 2
            }),
            apply_mode: "soft",
            published_by: alice_user_id,
        },
    )
    .await?;
    let store_001_device_auth_release_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: device_auth_config_id,
            deployment_instance_id: store_001_id,
            revision: "20260411.0103",
            content: "wifi:\n  ssid: store-wifi\n  password: super-secret\ncloud:\n  api_key: coffee-api-key\n",
            format: "yaml",
            change_summary: Some("Bootstrap device credentials"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 5,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_user_id,
        },
    )
    .await?;
    let store_001_vision_release_id = upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: vision_config_id,
            deployment_instance_id: store_001_id,
            revision: "20260409.0104",
            content: "enabled = true\ncamera_count = 2\n",
            format: "toml",
            change_summary: Some("Enable dual-camera vision"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_user_id,
        },
    )
    .await?;
    upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: store_002_id,
            revision: "20260408.0105",
            content: "log_level: info\npoll_interval_ms: 4000\n",
            format: "yaml",
            change_summary: Some("Initial store-002 release"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: admin_user_id,
        },
    )
    .await?;
    upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: staging_store_id,
            revision: "20260410.0106",
            content: "log_level: debug\npoll_interval_ms: 1500\n",
            format: "yaml",
            change_summary: Some("Staging tuning"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: alice_user_id,
        },
    )
    .await?;
    upsert_release(
        &mut tx,
        ReleaseSeed {
            project_id: billing_project_id,
            config_file_id: billing_main_config_id,
            deployment_instance_id: billing_store_id,
            revision: "20260407.0107",
            content: "timeout_ms: 2000\nretry_count: 3\n",
            format: "yaml",
            change_summary: Some("Initial billing defaults"),
            diff_summary: serde_json::json!({
                "is_initial": true,
                "has_changes": true,
                "added_lines": 2,
                "removed_lines": 0
            }),
            apply_mode: "soft",
            published_by: carol_user_id,
        },
    )
    .await?;

    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: template_id,
            content: "log_level: info\npoll_interval_ms: 4500\n",
            format: "yaml",
            version: 2,
            editor_user_id: admin_user_id,
        },
    )
    .await?;
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id: coffee_project_id,
            config_file_id: device_auth_config_id,
            deployment_instance_id: template_id,
            content: "wifi:\n  ssid: template-wifi\n  password: template-secret\ncloud:\n  api_key: template-api-key\n",
            format: "yaml",
            version: 1,
            editor_user_id: admin_user_id,
        },
    )
    .await?;
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id: coffee_project_id,
            config_file_id: vision_config_id,
            deployment_instance_id: template_id,
            content: "enabled = true\ncamera_count = 1\n",
            format: "toml",
            version: 1,
            editor_user_id: admin_user_id,
        },
    )
    .await?;
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id: coffee_project_id,
            config_file_id: main_config_id,
            deployment_instance_id: store_001_id,
            content: "log_level: error\npoll_interval_ms: 6000\n",
            format: "yaml",
            version: 3,
            editor_user_id: alice_user_id,
        },
    )
    .await?;
    upsert_draft(
        &mut tx,
        DraftSeed {
            project_id: coffee_project_id,
            config_file_id: vision_config_id,
            deployment_instance_id: store_002_id,
            content: "enabled = false\ncamera_count = 0\n",
            format: "toml",
            version: 1,
            editor_user_id: alice_user_id,
        },
    )
    .await?;

    upsert_credential(&mut tx, store_001_id, "default", STORE_001_TOKEN).await?;
    upsert_credential(&mut tx, store_002_id, "default", STORE_002_TOKEN).await?;
    upsert_credential(&mut tx, staging_store_id, "default", STAGING_TOKEN).await?;
    upsert_credential(
        &mut tx,
        inactive_store_id,
        "default",
        "mc_live_demo_inactive_store",
    )
    .await?;

    clear_seeded_sync_records(&mut tx).await?;
    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_001_id,
            config_file_id: main_config_id,
            release_id: Some(store_001_main_release_id),
            revision: Some("20260412.0102"),
            action: "apply",
            status: "success",
            message: Some("config applied"),
            detail: serde_json::json!({"duration_ms": 87, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T09:00:00Z",
        },
    )
    .await?;
    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_001_id,
            config_file_id: device_auth_config_id,
            release_id: Some(store_001_device_auth_release_id),
            revision: Some("20260411.0103"),
            action: "apply",
            status: "success",
            message: Some("secret config applied"),
            detail: serde_json::json!({"duration_ms": 42, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T09:01:00Z",
        },
    )
    .await?;
    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_002_id,
            config_file_id: main_config_id,
            release_id: None,
            revision: Some("20260408.0105"),
            action: "apply",
            status: "failed",
            message: Some("device-auth missing for publish gate"),
            detail: serde_json::json!({"seed_tag": SEED_TAG, "reason": "required_config_missing"}),
            reported_at: "2026-04-12T10:30:00Z",
        },
    )
    .await?;
    insert_sync_record(
        &mut tx,
        SyncRecordSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_001_id,
            config_file_id: vision_config_id,
            release_id: Some(store_001_vision_release_id),
            revision: Some("20260409.0104"),
            action: "apply",
            status: "success",
            message: Some("vision config applied"),
            detail: serde_json::json!({"duration_ms": 65, "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T09:03:00Z",
        },
    )
    .await?;

    upsert_heartbeat(
        &mut tx,
        HeartbeatSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_001_id,
            config_file_id: main_config_id,
            metadata: serde_json::json!({"status": "ready", "ip": "10.0.0.11", "version": "1.2.0", "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T09:10:00Z",
        },
    )
    .await?;
    upsert_heartbeat(
        &mut tx,
        HeartbeatSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_001_id,
            config_file_id: vision_config_id,
            metadata: serde_json::json!({"status": "ready", "ip": "10.0.0.12", "version": "1.0.3", "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T09:10:30Z",
        },
    )
    .await?;
    upsert_heartbeat(
        &mut tx,
        HeartbeatSeed {
            project_id: coffee_project_id,
            deployment_instance_id: store_002_id,
            config_file_id: main_config_id,
            metadata: serde_json::json!({"status": "degraded", "ip": "10.0.0.21", "version": "1.1.0", "seed_tag": SEED_TAG}),
            reported_at: "2026-04-12T10:32:00Z",
        },
    )
    .await?;

    clear_seeded_audit_logs(&mut tx).await?;
    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(coffee_project_id),
            user_id: Some(admin_user_id),
            action: "project.created",
            resource_type: "project",
            resource_id: coffee_project_id.to_string(),
            detail: Some(
                serde_json::json!({"seed_tag": SEED_TAG, "project_code": "demo-coffee-legacy"}),
            ),
            created_at: "2026-04-10T08:00:00Z",
        },
    )
    .await?;
    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(coffee_project_id),
            user_id: Some(admin_user_id),
            action: "deployment_instance.cloned",
            resource_type: "deployment_instance",
            resource_id: store_001_id.to_string(),
            detail: Some(serde_json::json!({"seed_tag": SEED_TAG, "source_kind": "draft", "template_source_id": template_id})),
            created_at: "2026-04-10T08:10:00Z",
        },
    )
    .await?;
    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: Some(coffee_project_id),
            user_id: Some(alice_user_id),
            action: "release.published",
            resource_type: "release",
            resource_id: store_001_main_release_id.to_string(),
            detail: Some(serde_json::json!({"seed_tag": SEED_TAG, "revision": "20260412.0102", "config_file_id": main_config_id})),
            created_at: "2026-04-12T09:00:00Z",
        },
    )
    .await?;
    insert_audit_log(
        &mut tx,
        AuditSeed {
            project_id: None,
            user_id: Some(admin_user_id),
            action: "auth.login.success",
            resource_type: "auth_session",
            resource_id: "seed-admin-login".to_owned(),
            detail: Some(serde_json::json!({"seed_tag": SEED_TAG, "username": ADMIN_USERNAME})),
            created_at: "2026-04-12T08:59:00Z",
        },
    )
    .await?;

    mark_setup_completed(&mut tx, admin_user_id).await?;

    tx.commit().await?;

    Ok(SeedSummary {
        coffee_project_code: "demo-coffee-legacy",
        billing_project_code: "demo-billing-service",
        store_001_token: STORE_001_TOKEN,
        store_002_token: STORE_002_TOKEN,
        staging_token: STAGING_TOKEN,
    })
}

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

struct AuditSeed {
    project_id: Option<i64>,
    user_id: Option<i64>,
    action: &'static str,
    resource_type: &'static str,
    resource_id: String,
    detail: Option<serde_json::Value>,
    created_at: &'static str,
}

async fn upsert_user(
    tx: &mut Transaction<'_, Postgres>,
    username: &str,
    password: &str,
    status: &str,
    is_platform_admin: bool,
) -> SeedResult<i64> {
    let password_hash =
        hash_password(password).map_err(|error| io::Error::other(error.into_body().message))?;

    let user_id = sqlx::query_scalar(
        r#"
        INSERT INTO users (
            username,
            password_hash,
            status,
            is_platform_admin,
            must_change_password,
            password_updated_at
        )
        VALUES ($1, $2, $3, $4, FALSE, NOW())
        ON CONFLICT (username)
        DO UPDATE SET
            password_hash = EXCLUDED.password_hash,
            status = EXCLUDED.status,
            is_platform_admin = EXCLUDED.is_platform_admin,
            must_change_password = EXCLUDED.must_change_password,
            password_updated_at = NOW(),
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .bind(status)
    .bind(is_platform_admin)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(user_id)
}

async fn mark_setup_completed(
    tx: &mut Transaction<'_, Postgres>,
    completed_by_user_id: i64,
) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO system_settings (
            id,
            setup_completed_at,
            setup_completed_by_user_id
        )
        VALUES (1, NOW(), $1)
        ON CONFLICT (id)
        DO UPDATE SET
            setup_completed_at = COALESCE(system_settings.setup_completed_at, NOW()),
            setup_completed_by_user_id = COALESCE(
                system_settings.setup_completed_by_user_id,
                EXCLUDED.setup_completed_by_user_id
            ),
            updated_at = NOW()
        "#,
    )
    .bind(completed_by_user_id)
    .execute(tx.as_mut())
    .await?;

    Ok(())
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

async fn upsert_config_file(
    tx: &mut Transaction<'_, Postgres>,
    seed: ConfigSeed<'_>,
) -> SeedResult<i64> {
    let config_file_id = sqlx::query_scalar(
        r#"
        INSERT INTO config_files (
            project_id,
            code,
            name,
            format,
            sensitivity,
            secret_paths,
            description,
            is_required,
            status
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

async fn upsert_project_environment(
    tx: &mut Transaction<'_, Postgres>,
    seed: ProjectEnvironmentSeed<'_>,
) -> SeedResult<i64> {
    let environment_id = sqlx::query_scalar(
        r#"
        INSERT INTO project_environments (
            project_id,
            code,
            name,
            description,
            status,
            sort_order
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

async fn upsert_deployment_instance(
    tx: &mut Transaction<'_, Postgres>,
    seed: DeploymentSeed<'_>,
) -> SeedResult<i64> {
    let deployment_id = sqlx::query_scalar(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment_id,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
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

async fn upsert_release(
    tx: &mut Transaction<'_, Postgres>,
    seed: ReleaseSeed<'_>,
) -> SeedResult<i64> {
    let release_id = sqlx::query_scalar(
        r#"
        INSERT INTO releases (
            project_id,
            config_file_id,
            deployment_instance_id,
            revision,
            content,
            content_hash,
            format,
            change_summary,
            diff_summary,
            apply_mode,
            published_by
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
            project_id,
            config_file_id,
            deployment_instance_id,
            content,
            content_hash,
            format,
            version,
            editor_user_id
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

async fn upsert_credential(
    tx: &mut Transaction<'_, Postgres>,
    deployment_instance_id: i64,
    credential_name: &str,
    token: &str,
) -> SeedResult {
    let token_hash = hash_bearer_token(token);

    // Demo deployment keys can be reused after local archive/delete experiments.
    // Remove stale credentials from tombstoned demo rows before reusing the same
    // deterministic demo token for the fresh replacement row.
    sqlx::query(
        r#"
        DELETE FROM deployment_credentials dc
        USING deployment_instances di
        WHERE dc.deployment_instance_id = di.id
          AND dc.token_hash = $1
          AND NOT (
            dc.deployment_instance_id = $2
            AND dc.credential_name = $3
          )
          AND (di.deleted_at IS NOT NULL OR di.is_archived = TRUE)
        "#,
    )
    .bind(&token_hash)
    .bind(deployment_instance_id)
    .bind(credential_name)
    .execute(tx.as_mut())
    .await?;

    sqlx::query(
        r#"
        INSERT INTO deployment_credentials (
            deployment_instance_id,
            credential_name,
            token_hash,
            status,
            last_used_at
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
    .bind(token_hash)
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
            project_id,
            deployment_instance_id,
            config_file_id,
            release_id,
            revision,
            action,
            status,
            message,
            detail,
            reported_at
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
            project_id,
            deployment_instance_id,
            config_file_id,
            metadata,
            reported_at
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

async fn insert_audit_log(tx: &mut Transaction<'_, Postgres>, seed: AuditSeed) -> SeedResult {
    sqlx::query(
        r#"
        INSERT INTO audit_logs (
            project_id,
            user_id,
            action,
            resource_type,
            resource_id,
            detail,
            created_at
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
