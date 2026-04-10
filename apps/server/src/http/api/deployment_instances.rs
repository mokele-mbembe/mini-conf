use crate::{
    audit::{AuditLogEntry, write_audit_log},
    auth::{deployment_token_preview, generate_deployment_token, hash_bearer_token},
    authorization::{ProjectRole, authenticate_user, require_project_role},
    error::ApiError,
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use schema::{
    deployment_instance::{
        DeploymentBundlePreviewResponse, DeploymentInstanceListResponse, DeploymentInstanceSummary,
        DeploymentPreviewItem, DeploymentTokenResetResponse,
    },
    open::{ConfigBundleItem, ConfigBundleResponse, ResolveDeployment},
};
use serde::Deserialize;
use sqlx::{Error as SqlxError, Row};

#[derive(Debug, Deserialize)]
pub(crate) struct ListDeploymentInstancesQuery {
    project_id: Option<i64>,
    environment: Option<String>,
    keyword: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateDeploymentInstanceRequest {
    project_id: Option<i64>,
    environment: Option<String>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    is_template: Option<bool>,
}

#[derive(Debug)]
struct ValidatedCreateDeploymentInstanceRequest {
    project_id: i64,
    environment: String,
    deployment_key: String,
    name: String,
    description: Option<String>,
    is_template: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UpdateDeploymentInstanceRequest {
    project_id: Option<i64>,
    environment: Option<String>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    is_template: Option<bool>,
    status: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateDeploymentInstanceRequest {
    project_id: i64,
    environment: String,
    deployment_key: String,
    name: String,
    description: Option<String>,
    is_template: bool,
    status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CloneDeploymentInstanceRequest {
    deployment_key: Option<String>,
    name: Option<String>,
    environment: Option<String>,
    description: Option<String>,
    clone_source: Option<String>,
}

#[derive(Debug)]
struct ValidatedCloneDeploymentInstanceRequest {
    deployment_key: String,
    name: String,
    environment: String,
    description: Option<String>,
}

#[derive(Debug)]
struct TemplateDeploymentContext {
    project_id: i64,
    is_template: bool,
}

#[derive(Debug)]
struct PreviewDeploymentContext {
    project_id: i64,
    project_code: String,
    environment: String,
    deployment_key: String,
    deployment_name: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/deployment-instances",
            get(list_deployment_instances).post(create_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}",
            get(get_deployment_instance).put(update_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}/clone",
            post(clone_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}/preview-bundle",
            get(preview_deployment_bundle),
        )
        .route(
            "/deployment-instances/{id}/token/reset",
            post(reset_deployment_token),
        )
}

#[utoipa::path(
    get,
    path = "/api/deployment-instances",
    tag = "admin",
    params(crate::openapi::ListDeploymentInstancesParams),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "List deployment instances", body = DeploymentInstanceListResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn list_deployment_instances(
    State(state): State<AppState>,
    Query(query): Query<ListDeploymentInstancesQuery>,
    headers: HeaderMap,
) -> Result<Json<DeploymentInstanceListResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            di.id,
            di.project_id,
            di.environment,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status
        FROM deployment_instances di
        JOIN project_members pm
          ON pm.project_id = di.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR di.project_id = $2)
          AND ($3::varchar IS NULL OR di.environment = $3)
          AND (
                $4::varchar IS NULL
                OR di.status = $4
          )
          AND (
                $5::varchar IS NULL
                OR di.deployment_key ILIKE '%' || $5 || '%'
                OR di.name ILIKE '%' || $5 || '%'
          )
        ORDER BY di.project_id ASC, di.environment ASC, di.deployment_key ASC, di.id ASC
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(normalize_optional(query.environment))
    .bind(normalize_optional(query.status))
    .bind(normalize_optional(query.keyword))
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(Json(DeploymentInstanceListResponse {
        items: rows.into_iter().map(map_deployment_row).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances",
    tag = "admin",
    request_body = crate::openapi::CreateDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Deployment instance created", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment key already exists within the project/environment", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn create_deployment_instance(
    State(state): State<AppState>,
    headers: HeaderMap,
    payload: Result<Json<CreateDeploymentInstanceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DeploymentInstanceSummary>), ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        payload.project_id,
        ProjectRole::Admin,
        "project_not_found",
        "project not found",
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            status
        )
        VALUES ($1, $2, $3, $4, $5, $6, 'active')
        RETURNING
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.environment)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.is_template)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_deployment_write_error)?;

    let summary = map_deployment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.created",
            resource_type: "deployment_instance",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": summary.id,
                "changed_fields": ["environment", "deployment_key", "name", "description", "is_template"]
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(summary)))
}

#[utoipa::path(
    get,
    path = "/api/deployment-instances/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance detail", body = DeploymentInstanceSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn get_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<DeploymentInstanceSummary>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let row = sqlx::query(
        r#"
        SELECT
            di.id,
            di.project_id,
            di.environment,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status
        FROM deployment_instances di
        JOIN project_members pm
          ON pm.project_id = di.project_id
         AND pm.user_id = $2
        WHERE di.id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(Json(map_deployment_row(row)))
}

#[utoipa::path(
    put,
    path = "/api/deployment-instances/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    request_body = crate::openapi::UpdateDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance updated", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Project or deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment key already exists within the project/environment", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn update_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<UpdateDeploymentInstanceRequest>, JsonRejection>,
) -> Result<Json<DeploymentInstanceSummary>, ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    let existing_project_id = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT project_id
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;
    require_project_role(
        pool,
        auth.user_id,
        existing_project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if payload.project_id != existing_project_id {
        require_project_role(
            pool,
            auth.user_id,
            payload.project_id,
            ProjectRole::Admin,
            "project_not_found",
            "project not found",
        )
        .await?;
    }

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        UPDATE deployment_instances
        SET
            project_id = $2,
            environment = $3,
            deployment_key = $4,
            name = $5,
            description = $6,
            is_template = $7,
            status = $8,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        "#,
    )
    .bind(id)
    .bind(payload.project_id)
    .bind(payload.environment)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.is_template)
    .bind(payload.status)
    .fetch_optional(&mut *tx)
    .await
    .map_err(map_deployment_write_error)?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    let summary = map_deployment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.updated",
            resource_type: "deployment_instance",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": summary.id,
                "changed_fields": ["environment", "deployment_key", "name", "description", "is_template", "status"]
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok(Json(summary))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/clone",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Template deployment instance ID")
    ),
    request_body = crate::openapi::CloneDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 201, description = "Deployment instance cloned from template", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Template deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Template deployment is invalid or deployment key conflicts", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn clone_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Result<Json<CloneDeploymentInstanceRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DeploymentInstanceSummary>), ApiError> {
    let Json(payload) =
        payload.map_err(|_| ApiError::bad_request("invalid_request", "invalid request body"))?;
    let payload = payload.validate()?;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;

    let template = load_template_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        template.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if !template.is_template {
        return Err(ApiError::conflict(
            "deployment_instance_not_template",
            "deployment instance is not a template",
        ));
    }

    let mut tx = pool.begin().await.map_err(|_| ApiError::internal())?;
    let row = sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        )
        VALUES ($1, $2, $3, $4, $5, FALSE, $6, 'active')
        RETURNING
            id,
            project_id,
            environment,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        "#,
    )
    .bind(template.project_id)
    .bind(&payload.environment)
    .bind(&payload.deployment_key)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_deployment_write_error)?;
    let cloned = map_deployment_row(row);

    clone_drafts_from_template(&mut tx, id, cloned.id, auth.user_id).await?;
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(cloned.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.cloned",
            resource_type: "deployment_instance",
            resource_id: cloned.id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": cloned.id,
                "source_deployment_instance_id": id,
                "source_kind": "draft"
            })),
        },
    )
    .await?;

    tx.commit().await.map_err(|_| ApiError::internal())?;

    Ok((StatusCode::CREATED, Json(cloned)))
}

#[utoipa::path(
    get,
    path = "/api/deployment-instances/{id}/preview-bundle",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment bundle preview", body = DeploymentBundlePreviewResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn preview_deployment_bundle(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<DeploymentBundlePreviewResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let context = load_preview_context(pool, id).await?;
    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Editor,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    let rows = sqlx::query(
        r#"
        SELECT
            cf.id AS config_file_id,
            cf.code,
            cf.name,
            cf.is_required,
            cf.format AS config_format,
            d.content AS draft_content,
            btrim(d.content_hash) AS draft_content_hash,
            d.version AS draft_version,
            r.content AS release_content,
            btrim(r.content_hash) AS release_content_hash,
            r.revision AS release_revision,
            r.format AS release_format
        FROM config_files cf
        LEFT JOIN drafts d
          ON d.config_file_id = cf.id
         AND d.deployment_instance_id = $2
        LEFT JOIN LATERAL (
            SELECT revision, content, btrim(content_hash) AS content_hash, format
            FROM releases
            WHERE deployment_instance_id = $2
              AND config_file_id = cf.id
            ORDER BY published_at DESC, id DESC
            LIMIT 1
        ) r ON TRUE
        WHERE cf.project_id = $1
          AND cf.status = 'active'
        ORDER BY cf.code ASC, cf.id ASC
        "#,
    )
    .bind(context.project_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    let mut items = Vec::with_capacity(rows.len());
    let mut bundle_items = Vec::new();

    for row in rows {
        let code: String = row.get("code");
        let name: String = row.get("name");
        let format: String = row.get("config_format");
        let is_required: bool = row.get("is_required");
        let draft_content = row.get::<Option<String>, _>("draft_content");
        let release_content = row.get::<Option<String>, _>("release_content");

        let (source, status, content, revision, content_hash, bundle_format) =
            if let Some(content) = draft_content {
                let version: i64 = row.get("draft_version");
                let content_hash: String = row.get("draft_content_hash");
                (
                    "draft".to_owned(),
                    "ready".to_owned(),
                    Some(content),
                    Some(format!("draft-v{version}")),
                    Some(content_hash),
                    format.clone(),
                )
            } else if let Some(content) = release_content {
                let revision: String = row.get("release_revision");
                let content_hash: String = row.get("release_content_hash");
                let release_format: String = row.get("release_format");
                (
                    "latest_release".to_owned(),
                    "ready".to_owned(),
                    Some(content),
                    Some(revision),
                    Some(content_hash),
                    release_format,
                )
            } else if is_required {
                (
                    "none".to_owned(),
                    "missing_required".to_owned(),
                    None,
                    None,
                    None,
                    format.clone(),
                )
            } else {
                (
                    "none".to_owned(),
                    "missing_optional".to_owned(),
                    None,
                    None,
                    None,
                    format.clone(),
                )
            };

        if let (Some(content), Some(revision), Some(content_hash)) =
            (content.clone(), revision.clone(), content_hash.clone())
        {
            bundle_items.push(ConfigBundleItem {
                config: code.clone(),
                revision,
                content_hash,
                format: bundle_format,
                content,
            });
        }

        items.push(DeploymentPreviewItem {
            config_file_id: row.get("config_file_id"),
            code,
            name,
            is_required,
            source,
            status,
            format,
            content,
            revision,
        });
    }

    Ok(Json(DeploymentBundlePreviewResponse {
        deployment_instance_id: id,
        items,
        open_bundle_preview: ConfigBundleResponse {
            project: context.project_code,
            environment: context.environment,
            deployment: ResolveDeployment {
                key: context.deployment_key,
                name: context.deployment_name,
            },
            configs: bundle_items,
        },
    }))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/token/reset",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment token rotated", body = DeploymentTokenResetResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn reset_deployment_token(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Json<DeploymentTokenResetResponse>, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    let (project_id, deployment_status) = load_deployment_project(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if deployment_status != "active" {
        return Err(ApiError::conflict(
            "deployment_instance_inactive",
            "deployment instance is not active",
        ));
    }

    let token = generate_deployment_token();
    let token_hash = hash_bearer_token(&token);
    let credential_name = upsert_default_deployment_credential(pool, id, &token_hash).await?;
    let response = DeploymentTokenResetResponse {
        deployment_instance_id: id,
        credential_name,
        token_preview: deployment_token_preview(&token),
        token,
    };
    write_audit_log(
        pool,
        AuditLogEntry {
            project_id: Some(project_id),
            user_id: Some(auth.user_id),
            action: "deployment_token.reset",
            resource_type: "deployment_instance",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": id,
                "token_preview": response.token_preview,
            })),
        },
    )
    .await?;

    Ok(Json(response))
}

impl CreateDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedCreateDeploymentInstanceRequest, ApiError> {
        Ok(ValidatedCreateDeploymentInstanceRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            is_template: self.is_template.unwrap_or(false),
        })
    }
}

impl UpdateDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedUpdateDeploymentInstanceRequest, ApiError> {
        Ok(ValidatedUpdateDeploymentInstanceRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            environment: required(self.environment, "environment")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
            is_template: self.is_template.unwrap_or(false),
            status: validate_status(self.status)?,
        })
    }
}

impl CloneDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedCloneDeploymentInstanceRequest, ApiError> {
        validate_clone_source(self.clone_source)?;

        Ok(ValidatedCloneDeploymentInstanceRequest {
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            environment: required(self.environment, "environment")?,
            description: normalize_optional(self.description),
        })
    }
}

fn map_deployment_row(row: sqlx::postgres::PgRow) -> DeploymentInstanceSummary {
    DeploymentInstanceSummary {
        id: row.get("id"),
        project_id: row.get("project_id"),
        environment: row.get("environment"),
        deployment_key: row.get("deployment_key"),
        name: row.get("name"),
        description: row.get("description"),
        is_template: row.get("is_template"),
        template_source_id: row.get("template_source_id"),
        status: row.get("status"),
    }
}

fn map_deployment_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error {
        if database_error.constraint()
            == Some("deployment_instances_project_id_environment_deployment_key_key")
        {
            return ApiError::conflict(
                "deployment_key_conflict",
                "deployment key already exists in project environment",
            );
        }

        if database_error.constraint() == Some("deployment_instances_project_id_fkey") {
            return ApiError::not_found_with("project_not_found", "project not found");
        }
    }

    ApiError::internal()
}

async fn load_template_context(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<TemplateDeploymentContext, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT project_id, is_template
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(TemplateDeploymentContext {
        project_id: row.get("project_id"),
        is_template: row.get("is_template"),
    })
}

async fn load_deployment_project(pool: &sqlx::PgPool, id: i64) -> Result<(i64, String), ApiError> {
    let row = sqlx::query(
        r#"
        SELECT project_id, status
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok((row.get("project_id"), row.get("status")))
}

async fn upsert_default_deployment_credential(
    pool: &sqlx::PgPool,
    deployment_id: i64,
    token_hash: &str,
) -> Result<String, ApiError> {
    let row = sqlx::query(
        r#"
        INSERT INTO deployment_credentials (
            deployment_instance_id,
            credential_name,
            token_hash,
            status,
            last_used_at
        )
        VALUES ($1, 'default', $2, 'active', NULL)
        ON CONFLICT (deployment_instance_id, credential_name)
        DO UPDATE SET
            token_hash = EXCLUDED.token_hash,
            status = 'active',
            last_used_at = NULL,
            updated_at = NOW()
        RETURNING credential_name
        "#,
    )
    .bind(deployment_id)
    .bind(token_hash)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(row.get("credential_name"))
}

async fn load_preview_context(
    pool: &sqlx::PgPool,
    deployment_id: i64,
) -> Result<PreviewDeploymentContext, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            di.project_id,
            p.code AS project_code,
            di.environment,
            di.deployment_key,
            di.name AS deployment_name
        FROM deployment_instances di
        JOIN projects p ON p.id = di.project_id
        WHERE di.id = $1
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(PreviewDeploymentContext {
        project_id: row.get("project_id"),
        project_code: row.get("project_code"),
        environment: row.get("environment"),
        deployment_key: row.get("deployment_key"),
        deployment_name: row.get("deployment_name"),
    })
}

async fn clone_drafts_from_template(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    source_deployment_id: i64,
    target_deployment_id: i64,
    editor_user_id: i64,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO drafts (
            project_id,
            config_file_id,
            deployment_instance_id,
            content,
            content_hash,
            format,
            schema_version,
            version,
            editor_user_id
        )
        SELECT
            project_id,
            config_file_id,
            $2,
            content,
            content_hash,
            format,
            schema_version,
            1,
            $3
        FROM drafts
        WHERE deployment_instance_id = $1
        "#,
    )
    .bind(source_deployment_id)
    .bind(target_deployment_id)
    .bind(editor_user_id)
    .execute(&mut **tx)
    .await
    .map_err(|_| ApiError::internal())?;

    Ok(())
}

fn required(value: Option<String>, field: &'static str) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message(field),
        ));
    };

    Ok(value)
}

fn required_i64(value: Option<i64>, field: &'static str) -> Result<i64, ApiError> {
    value.ok_or_else(|| ApiError::bad_request("invalid_request", invalid_body_message(field)))
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

fn invalid_body_message(field: &'static str) -> &'static str {
    match field {
        "project_id" => "missing required body field: project_id",
        "environment" => "missing required body field: environment",
        "deployment_key" => "missing required body field: deployment_key",
        "name" => "missing required body field: name",
        "clone_source" => "missing required body field: clone_source",
        "status" => "missing required body field: status",
        _ => "missing required body field",
    }
}

fn validate_status(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message("status"),
        ));
    };

    match value.as_str() {
        "active" | "archived" => Ok(value),
        _ => Err(ApiError::bad_request(
            "invalid_request",
            "invalid deployment instance status",
        )),
    }
}

fn validate_clone_source(value: Option<String>) -> Result<String, ApiError> {
    let Some(value) = normalize_optional(value) else {
        return Err(ApiError::bad_request(
            "invalid_request",
            invalid_body_message("clone_source"),
        ));
    };

    if value == "draft" {
        Ok(value)
    } else {
        Err(ApiError::bad_request(
            "invalid_request",
            "invalid deployment clone source",
        ))
    }
}
