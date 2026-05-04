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
    environment_id: Option<i64>,
    keyword: Option<String>,
    status: Option<String>,
    visibility_filter: Option<String>,
    is_template: Option<bool>,
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeploymentVisibilityFilter {
    Current,
    Archived,
    All,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CreateDeploymentInstanceRequest {
    project_id: Option<i64>,
    environment_id: Option<i64>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
    is_template: Option<bool>,
}

#[derive(Debug)]
struct ValidatedCreateDeploymentInstanceRequest {
    project_id: i64,
    environment_id: i64,
    deployment_key: String,
    name: String,
    description: Option<String>,
    is_template: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateDeploymentInstanceRequest {
    environment_id: Option<i64>,
    deployment_key: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct ValidatedUpdateDeploymentInstanceRequest {
    environment_id: i64,
    deployment_key: String,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CloneDeploymentInstanceRequest {
    deployment_key: Option<String>,
    name: Option<String>,
    environment_id: Option<i64>,
    description: Option<String>,
    clone_source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArchiveDeploymentInstanceRequest {
    reason: Option<String>,
}

#[derive(Debug)]
struct ValidatedArchiveDeploymentInstanceRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteDeploymentInstanceRequest {
    reason: Option<String>,
}

#[derive(Debug)]
struct ValidatedDeleteDeploymentInstanceRequest {
    reason: Option<String>,
}

#[derive(Debug)]
struct ValidatedCloneDeploymentInstanceRequest {
    deployment_key: String,
    name: String,
    environment_id: i64,
    description: Option<String>,
}

#[derive(Debug)]
struct TemplateDeploymentContext {
    project_id: i64,
    is_template: bool,
    is_archived: bool,
    deleted_at: Option<String>,
}

#[derive(Debug)]
struct DeploymentLifecycleContext {
    project_id: i64,
    deployment_uid: String,
    is_template: bool,
    status: String,
    is_archived: bool,
    deleted_at: Option<String>,
}

#[derive(Debug)]
struct PreviewDeploymentContext {
    project_id: i64,
    project_code: String,
    environment_code: String,
    deployment_key: String,
    deployment_name: String,
}

#[derive(Debug)]
struct ProjectEnvironmentAssignmentContext {
    code: String,
    name: String,
    status: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/deployment-instances",
            get(list_deployment_instances).post(create_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}",
            get(get_deployment_instance)
                .put(update_deployment_instance)
                .delete(delete_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}/archive",
            post(archive_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}/restore",
            post(restore_deployment_instance),
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
        .route(
            "/deployment-instances/{id}/activate",
            post(activate_deployment_instance),
        )
        .route(
            "/deployment-instances/{id}/deactivate",
            post(deactivate_deployment_instance),
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
    let (page, page_size) = validate_page(query.page, query.page_size)?;
    let offset = (page - 1) * page_size;
    let visibility_filter = parse_visibility_filter(query.visibility_filter.as_deref())?;

    let total = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM deployment_instances di
        JOIN project_environments pe
          ON pe.project_id = di.project_id
         AND pe.id = di.environment_id
        JOIN project_members pm
          ON pm.project_id = di.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR di.project_id = $2)
          AND ($3::bigint IS NULL OR di.environment_id = $3)
          AND ($4::varchar IS NULL OR di.status = $4)
            AND di.deleted_at IS NULL
            AND (
                ($7::varchar = 'current' AND di.is_archived = FALSE)
                OR ($7::varchar = 'archived' AND di.is_archived = TRUE)
                OR ($7::varchar = 'all')
            )
          AND (
                $5::varchar IS NULL
                OR di.deployment_key ILIKE '%' || $5 || '%'
                OR di.name ILIKE '%' || $5 || '%'
          )
          AND ($6::boolean IS NULL OR di.is_template = $6)
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(query.environment_id)
    .bind(normalize_optional(query.status.clone()))
    .bind(normalize_optional(query.keyword.clone()))
    .bind(query.is_template)
    .bind(visibility_filter.as_str())
    .fetch_one(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to list deployment instances"))?;

    let rows = sqlx::query(
        r#"
        SELECT
            di.id,
            btrim(di.deployment_uid::text) AS deployment_uid,
            di.project_id,
            di.environment_id,
            pe.code AS environment_code,
            pe.name AS environment_name,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status,
            di.is_archived,
            to_char(di.archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            di.archived_by,
            di.archive_reason,
            to_char(di.deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            di.deleted_by,
            di.delete_reason
        FROM deployment_instances di
        JOIN project_environments pe
          ON pe.project_id = di.project_id
         AND pe.id = di.environment_id
        JOIN project_members pm
          ON pm.project_id = di.project_id
         AND pm.user_id = $1
        WHERE ($2::bigint IS NULL OR di.project_id = $2)
          AND ($3::bigint IS NULL OR di.environment_id = $3)
          AND (
                $4::varchar IS NULL
                OR di.status = $4
          )
            AND di.deleted_at IS NULL
            AND (
                ($7::varchar = 'current' AND di.is_archived = FALSE)
                OR ($7::varchar = 'archived' AND di.is_archived = TRUE)
                OR ($7::varchar = 'all')
            )
          AND (
                $5::varchar IS NULL
                OR di.deployment_key ILIKE '%' || $5 || '%'
                OR di.name ILIKE '%' || $5 || '%'
        )
          AND ($6::boolean IS NULL OR di.is_template = $6)
        ORDER BY di.project_id ASC, pe.code ASC, di.deployment_key ASC, di.id ASC
        LIMIT $8 OFFSET $9
        "#,
    )
    .bind(auth.user_id)
    .bind(query.project_id)
    .bind(query.environment_id)
    .bind(normalize_optional(query.status))
    .bind(normalize_optional(query.keyword))
    .bind(query.is_template)
    .bind(visibility_filter.as_str())
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to list deployment instances"))?;

    Ok(Json(DeploymentInstanceListResponse {
        items: rows.into_iter().map(map_deployment_row).collect(),
        total,
        page,
        page_size,
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
        (status = 409, description = "Deployment key already exists within the project/environment or the environment is inactive", body = crate::error::ErrorResponse),
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
    let environment =
        load_project_environment_for_assignment(pool, payload.project_id, payload.environment_id)
            .await?;
    if environment.status != "active" {
        return Err(ApiError::conflict(
            "project_environment_inactive",
            "project environment is inactive",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to create deployment instance"))?;
    let row = sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            deployment_uid,
            project_id,
            environment_id,
            deployment_key,
            name,
            description,
            is_template,
            status
        )
        VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, 'inactive')
        RETURNING
            id,
            btrim(deployment_uid::text) AS deployment_uid,
            project_id,
            environment_id,
            $7::varchar AS environment_code,
            $8::varchar AS environment_name,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status,
            is_archived,
            to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            archived_by,
            archive_reason,
            to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            deleted_by,
            delete_reason
        "#,
    )
    .bind(payload.project_id)
    .bind(payload.environment_id)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(payload.is_template)
    .bind(environment.code)
    .bind(environment.name)
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
                "deployment_uid": summary.deployment_uid,
                "changed_fields": ["environment_id", "deployment_key", "name", "description", "is_template"]
            })),
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to create deployment instance"))?;

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
            btrim(di.deployment_uid::text) AS deployment_uid,
            di.project_id,
            di.environment_id,
            pe.code AS environment_code,
            pe.name AS environment_name,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status,
            di.is_archived,
            to_char(di.archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            di.archived_by,
            di.archive_reason,
            to_char(di.deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            di.deleted_by,
            di.delete_reason
        FROM deployment_instances di
        JOIN project_environments pe
          ON pe.project_id = di.project_id
         AND pe.id = di.environment_id
        JOIN project_members pm
          ON pm.project_id = di.project_id
         AND pm.user_id = $2
        WHERE di.id = $1
          AND di.deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to get deployment instance"))?
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
        (status = 409, description = "Deployment key already exists within the project/environment or the environment is inactive", body = crate::error::ErrorResponse),
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
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    ensure_not_archived_or_deleted(&context)?;
    let environment =
        load_project_environment_for_assignment(pool, context.project_id, payload.environment_id)
            .await?;
    if environment.status != "active" {
        return Err(ApiError::conflict(
            "project_environment_inactive",
            "project environment is inactive",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to update deployment instance"))?;
    let row = sqlx::query(
        r#"
        UPDATE deployment_instances
        SET
            environment_id = $2,
            deployment_key = $3,
            name = $4,
            description = $5,
            updated_at = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
        RETURNING
            id,
            btrim(deployment_uid::text) AS deployment_uid,
            project_id,
            environment_id,
            $6::varchar AS environment_code,
            $7::varchar AS environment_name,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status,
            is_archived,
            to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            archived_by,
            archive_reason,
            to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            deleted_by,
            delete_reason
        "#,
    )
    .bind(id)
    .bind(payload.environment_id)
    .bind(payload.deployment_key)
    .bind(payload.name)
    .bind(payload.description)
    .bind(environment.code)
    .bind(environment.name)
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
                "deployment_uid": summary.deployment_uid,
                "changed_fields": ["environment_id", "deployment_key", "name", "description"]
            })),
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to update deployment instance"))?;

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
    if template.deleted_at.is_some() {
        return Err(ApiError::conflict(
            "deployment_instance_deleted",
            "deployment instance has been deleted",
        ));
    }
    if template.is_archived {
        return Err(ApiError::conflict(
            "deployment_instance_archived",
            "deployment instance is archived",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to clone deployment instance"))?;
    let environment =
        load_project_environment_for_assignment(pool, template.project_id, payload.environment_id)
            .await?;
    if environment.status != "active" {
        return Err(ApiError::conflict(
            "project_environment_inactive",
            "project environment is inactive",
        ));
    }
    let row = sqlx::query(
        r#"
        INSERT INTO deployment_instances (
            deployment_uid,
            project_id,
            environment_id,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status
        )
        VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, FALSE, $6, 'inactive')
        RETURNING
            id,
            btrim(deployment_uid::text) AS deployment_uid,
            project_id,
            environment_id,
            $7::varchar AS environment_code,
            $8::varchar AS environment_name,
            deployment_key,
            name,
            description,
            is_template,
            template_source_id,
            status,
            is_archived,
            to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            archived_by,
            archive_reason,
            to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            deleted_by,
            delete_reason
        "#,
    )
    .bind(template.project_id)
    .bind(payload.environment_id)
    .bind(&payload.deployment_key)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(id)
    .bind(environment.code)
    .bind(environment.name)
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
                "deployment_uid": cloned.deployment_uid,
                "source_deployment_instance_id": id,
                "source_kind": "draft"
            })),
        },
    )
    .await?;

    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to clone deployment instance"))?;

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

    let lifecycle = load_deployment_context(pool, id).await?;
    let auth = authenticate_user(pool, &headers).await?;
    require_project_role(
        pool,
        auth.user_id,
        lifecycle.project_id,
        ProjectRole::Editor,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    ensure_not_archived_or_deleted(&lifecycle)?;
    let context = load_preview_context(pool, id).await?;
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
    .map_err(|error| ApiError::internal_with(error, "failed to preview deployment bundle"))?;

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
            (&content, &revision, &content_hash)
        {
            bundle_items.push(ConfigBundleItem {
                config: code.clone(),
                revision: revision.clone(),
                content_hash: content_hash.clone(),
                format: bundle_format,
                content: content.clone(),
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
            environment: context.environment_code,
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
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    ensure_not_archived_or_deleted(&context)?;
    if context.is_template {
        return Err(ApiError::conflict(
            "deployment_instance_template_token_forbidden",
            "template deployment instances cannot reset tokens",
        ));
    }
    if context.status != "active" {
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
            project_id: Some(context.project_id),
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

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/activate",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment activated and token issued", body = DeploymentTokenResetResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment instance cannot be activated", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn activate_deployment_instance(
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
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    ensure_not_archived_or_deleted(&context)?;
    if context.is_template {
        return Err(ApiError::conflict(
            "deployment_instance_template_activate_forbidden",
            "template deployment instances cannot be activated",
        ));
    }
    if context.status != "inactive" {
        return Err(ApiError::conflict(
            "deployment_instance_activate_conflict",
            "deployment instance must be inactive before activation",
        ));
    }

    let mut tx = pool.begin().await.map_err(|error| {
        ApiError::internal_with(error, "failed to activate deployment instance")
    })?;
    sqlx::query(
        r#"
        UPDATE deployment_instances
        SET status = 'active', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to activate deployment instance"))?;

    let token = generate_deployment_token();
    let token_hash = hash_bearer_token(&token);
    let credential_name =
        upsert_default_deployment_credential_in_tx(&mut tx, id, &token_hash).await?;
    let response = DeploymentTokenResetResponse {
        deployment_instance_id: id,
        credential_name,
        token_preview: deployment_token_preview(&token),
        token,
    };
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(context.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.activated",
            resource_type: "deployment_instance",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": id,
                "token_preview": response.token_preview,
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|error| {
        ApiError::internal_with(error, "failed to activate deployment instance")
    })?;

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/deactivate",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Deployment deactivated"),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment instance cannot be deactivated", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn deactivate_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    ensure_not_archived_or_deleted(&context)?;
    if context.is_template {
        return Err(ApiError::conflict(
            "deployment_instance_template_deactivate_forbidden",
            "template deployment instances cannot be deactivated",
        ));
    }
    if context.status != "active" {
        return Err(ApiError::conflict(
            "deployment_instance_deactivate_conflict",
            "deployment instance must be active before deactivation",
        ));
    }

    let mut tx = pool.begin().await.map_err(|error| {
        ApiError::internal_with(error, "failed to deactivate deployment instance")
    })?;
    sqlx::query(
        r#"
        UPDATE deployment_instances
        SET status = 'inactive', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to deactivate deployment instance"))?;
    sqlx::query(
        r#"
        UPDATE deployment_credentials
        SET status = 'inactive', updated_at = NOW()
        WHERE deployment_instance_id = $1
          AND credential_name = 'default'
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to deactivate deployment instance"))?;
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(context.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.deactivated",
            resource_type: "deployment_instance",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": id,
            })),
        },
    )
    .await?;
    tx.commit().await.map_err(|error| {
        ApiError::internal_with(error, "failed to deactivate deployment instance")
    })?;

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/archive",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    request_body = crate::openapi::ArchiveDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance archived", body = DeploymentInstanceSummary),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment instance cannot be archived", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn archive_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Option<Json<ArchiveDeploymentInstanceRequest>>,
) -> Result<Json<DeploymentInstanceSummary>, ApiError> {
    let payload = payload
        .map(|Json(payload)| payload.validate())
        .unwrap_or(ValidatedArchiveDeploymentInstanceRequest { reason: None });

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if context.deleted_at.is_some() {
        return Err(ApiError::conflict(
            "deployment_instance_deleted",
            "deployment instance has been deleted",
        ));
    }
    if context.is_archived {
        return Err(ApiError::conflict(
            "deployment_instance_archive_conflict",
            "deployment instance is already archived",
        ));
    }
    if context.status != "inactive" {
        return Err(ApiError::conflict(
            "deployment_instance_archive_conflict",
            "deployment instance must be inactive before archive",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to archive deployment instance"))?;
    let row = sqlx::query(
        r#"
        UPDATE deployment_instances di
        SET
            is_archived = TRUE,
            archived_at = NOW(),
            archived_by = $2,
            archive_reason = $3,
            updated_at = NOW()
        FROM project_environments pe
        WHERE di.id = $1
          AND di.deleted_at IS NULL
          AND di.is_archived = FALSE
          AND di.status = 'inactive'
          AND pe.project_id = di.project_id
          AND pe.id = di.environment_id
        RETURNING
            di.id,
            btrim(di.deployment_uid::text) AS deployment_uid,
            di.project_id,
            di.environment_id,
            pe.code AS environment_code,
            pe.name AS environment_name,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status,
            di.is_archived,
            to_char(di.archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            di.archived_by,
            di.archive_reason,
            to_char(di.deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            di.deleted_by,
            di.delete_reason
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(payload.reason)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to archive deployment instance"))?
    .ok_or_else(|| {
        ApiError::conflict(
            "deployment_instance_archive_conflict",
            "deployment instance could not be archived due to a concurrent state change",
        )
    })?;

    let summary = map_deployment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.archived",
            resource_type: "deployment_instance",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": summary.id,
                "deployment_uid": summary.deployment_uid,
                "archive_reason": summary.archive_reason,
            })),
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to archive deployment instance"))?;

    Ok(Json(summary))
}

#[utoipa::path(
    post,
    path = "/api/deployment-instances/{id}/restore",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 200, description = "Deployment instance restored", body = DeploymentInstanceSummary),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment instance cannot be restored", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn restore_deployment_instance(
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
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if context.deleted_at.is_some() {
        return Err(ApiError::conflict(
            "deployment_instance_deleted",
            "deployment instance has been deleted",
        ));
    }
    if !context.is_archived {
        return Err(ApiError::conflict(
            "deployment_instance_restore_conflict",
            "deployment instance is not archived",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to restore deployment instance"))?;
    let row = sqlx::query(
        r#"
        UPDATE deployment_instances di
        SET
            is_archived = FALSE,
            archived_at = NULL,
            archived_by = NULL,
            archive_reason = NULL,
            status = 'inactive',
            updated_at = NOW()
        FROM project_environments pe
        WHERE di.id = $1
          AND di.deleted_at IS NULL
          AND di.is_archived = TRUE
          AND pe.project_id = di.project_id
          AND pe.id = di.environment_id
        RETURNING
            di.id,
            btrim(di.deployment_uid::text) AS deployment_uid,
            di.project_id,
            di.environment_id,
            pe.code AS environment_code,
            pe.name AS environment_name,
            di.deployment_key,
            di.name,
            di.description,
            di.is_template,
            di.template_source_id,
            di.status,
            di.is_archived,
            to_char(di.archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS archived_at,
            di.archived_by,
            di.archive_reason,
            to_char(di.deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at,
            di.deleted_by,
            di.delete_reason
        "#,
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to restore deployment instance"))?
    .ok_or_else(|| {
        ApiError::conflict(
            "deployment_instance_restore_conflict",
            "deployment instance could not be restored due to a concurrent state change",
        )
    })?;

    let summary = map_deployment_row(row);
    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(summary.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.restored",
            resource_type: "deployment_instance",
            resource_id: summary.id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": summary.id,
                "deployment_uid": summary.deployment_uid,
            })),
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to restore deployment instance"))?;

    Ok(Json(summary))
}

#[utoipa::path(
    delete,
    path = "/api/deployment-instances/{id}",
    tag = "admin",
    params(
        ("id" = i64, Path, description = "Deployment instance ID")
    ),
    request_body = crate::openapi::DeleteDeploymentInstanceRequestBody,
    security(
        ("session_auth" = [])
    ),
    responses(
        (status = 204, description = "Deployment instance deleted"),
        (status = 400, description = "Invalid request body", body = crate::error::ErrorResponse),
        (status = 401, description = "Missing or expired admin session", body = crate::error::ErrorResponse),
        (status = 404, description = "Deployment instance not found", body = crate::error::ErrorResponse),
        (status = 409, description = "Deployment instance cannot be deleted", body = crate::error::ErrorResponse),
        (status = 503, description = "Database bootstrap disabled", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse),
    )
)]
pub(crate) async fn delete_deployment_instance(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    payload: Option<Json<DeleteDeploymentInstanceRequest>>,
) -> Result<StatusCode, ApiError> {
    let payload = payload
        .map(|Json(payload)| payload.validate())
        .unwrap_or(ValidatedDeleteDeploymentInstanceRequest { reason: None });
    let delete_reason = payload.reason;

    let Some(pool) = state.db_pool() else {
        return Err(ApiError::service_unavailable(
            "database_unavailable",
            "Database bootstrap is disabled",
        ));
    };

    let auth = authenticate_user(pool, &headers).await?;
    let context = load_deployment_context(pool, id).await?;
    require_project_role(
        pool,
        auth.user_id,
        context.project_id,
        ProjectRole::Admin,
        "deployment_instance_not_found",
        "deployment instance not found",
    )
    .await?;
    if context.deleted_at.is_some() {
        return Err(ApiError::conflict(
            "deployment_instance_deleted",
            "deployment instance has been deleted",
        ));
    }
    if !context.is_archived || context.status != "inactive" {
        return Err(ApiError::conflict(
            "deployment_instance_delete_conflict",
            "deployment instance must be archived and inactive before delete",
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;
    let tombstone_result = sqlx::query(
        r#"
        UPDATE deployment_instances
        SET
            deleted_at = NOW(),
            deleted_by = $2,
            delete_reason = $3,
            updated_at = NOW()
        WHERE id = $1
          AND deleted_at IS NULL
          AND is_archived = TRUE
          AND status = 'inactive'
        "#,
    )
    .bind(id)
    .bind(auth.user_id)
    .bind(delete_reason.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;
    if tombstone_result.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "deployment_instance_delete_conflict",
            "deployment instance could not be deleted due to a concurrent state change",
        ));
    }

    sqlx::query(
        r#"
        UPDATE deployment_credentials
        SET status = 'inactive', updated_at = NOW()
        WHERE deployment_instance_id = $1
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;

    sqlx::query(
        r#"
        DELETE FROM drafts
        WHERE deployment_instance_id = $1
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;

    sqlx::query(
        r#"
        UPDATE draft_saved_versions
        SET deleted_at = COALESCE(deleted_at, NOW()), updated_at = NOW()
        WHERE deployment_instance_id = $1
          AND deleted_at IS NULL
        "#,
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;

    write_audit_log(
        &mut *tx,
        AuditLogEntry {
            project_id: Some(context.project_id),
            user_id: Some(auth.user_id),
            action: "deployment_instance.deleted",
            resource_type: "deployment_instance",
            resource_id: id.to_string(),
            detail: Some(serde_json::json!({
                "deployment_instance_id": id,
                "deployment_uid": context.deployment_uid,
                "delete_reason": delete_reason,
            })),
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| ApiError::internal_with(error, "failed to delete deployment instance"))?;

    Ok(StatusCode::NO_CONTENT)
}

impl CreateDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedCreateDeploymentInstanceRequest, ApiError> {
        Ok(ValidatedCreateDeploymentInstanceRequest {
            project_id: required_i64(self.project_id, "project_id")?,
            environment_id: required_i64(self.environment_id, "environment_id")?,
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
            environment_id: required_i64(self.environment_id, "environment_id")?,
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            description: normalize_optional(self.description),
        })
    }
}

impl CloneDeploymentInstanceRequest {
    fn validate(self) -> Result<ValidatedCloneDeploymentInstanceRequest, ApiError> {
        validate_clone_source(self.clone_source)?;

        Ok(ValidatedCloneDeploymentInstanceRequest {
            deployment_key: required(self.deployment_key, "deployment_key")?,
            name: required(self.name, "name")?,
            environment_id: required_i64(self.environment_id, "environment_id")?,
            description: normalize_optional(self.description),
        })
    }
}

impl ArchiveDeploymentInstanceRequest {
    fn validate(self) -> ValidatedArchiveDeploymentInstanceRequest {
        ValidatedArchiveDeploymentInstanceRequest {
            reason: normalize_optional(self.reason),
        }
    }
}

impl DeleteDeploymentInstanceRequest {
    fn validate(self) -> ValidatedDeleteDeploymentInstanceRequest {
        ValidatedDeleteDeploymentInstanceRequest {
            reason: normalize_optional(self.reason),
        }
    }
}

impl DeploymentVisibilityFilter {
    fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Archived => "archived",
            Self::All => "all",
        }
    }
}

fn parse_visibility_filter(value: Option<&str>) -> Result<DeploymentVisibilityFilter, ApiError> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(DeploymentVisibilityFilter::Current),
        Some("current") => Ok(DeploymentVisibilityFilter::Current),
        Some("archived") => Ok(DeploymentVisibilityFilter::Archived),
        Some("all") => Ok(DeploymentVisibilityFilter::All),
        Some(_) => Err(ApiError::bad_request(
            "invalid_request",
            "visibility_filter must be one of: current, archived, all",
        )),
    }
}

fn ensure_not_archived_or_deleted(context: &DeploymentLifecycleContext) -> Result<(), ApiError> {
    if context.deleted_at.is_some() {
        return Err(ApiError::conflict(
            "deployment_instance_deleted",
            "deployment instance has been deleted",
        ));
    }
    if context.is_archived {
        return Err(ApiError::conflict(
            "deployment_instance_archived",
            "deployment instance is archived",
        ));
    }

    Ok(())
}

fn map_deployment_row(row: sqlx::postgres::PgRow) -> DeploymentInstanceSummary {
    DeploymentInstanceSummary {
        id: row.get("id"),
        deployment_uid: row.get("deployment_uid"),
        project_id: row.get("project_id"),
        environment_id: row.get("environment_id"),
        environment_code: row.get("environment_code"),
        environment_name: row.get("environment_name"),
        deployment_key: row.get("deployment_key"),
        name: row.get("name"),
        description: row.get("description"),
        is_template: row.get("is_template"),
        template_source_id: row.get("template_source_id"),
        status: row.get("status"),
        is_archived: row.get("is_archived"),
        archived_at: row.get("archived_at"),
        archived_by: row.get("archived_by"),
        archive_reason: row.get("archive_reason"),
        deleted_at: row.get("deleted_at"),
        deleted_by: row.get("deleted_by"),
        delete_reason: row.get("delete_reason"),
    }
}

fn map_deployment_write_error(error: SqlxError) -> ApiError {
    if let SqlxError::Database(database_error) = &error {
        if database_error.constraint().is_some_and(|constraint| {
            constraint.starts_with("deployment_instances_project_id_environment")
                || constraint == "deployment_instances_live_key_unique"
        }) {
            return ApiError::conflict(
                "deployment_key_conflict",
                "deployment key already exists in project environment",
            );
        }

        if database_error.constraint() == Some("deployment_instances_project_id_fkey") {
            return ApiError::not_found_with("project_not_found", "project not found");
        }

        if database_error.constraint()
            == Some("deployment_instances_project_id_environment_id_fkey")
        {
            return ApiError::not_found_with(
                "project_environment_not_found",
                "project environment not found",
            );
        }
    }

    ApiError::internal_with(error, "failed to write deployment instance")
}

async fn load_project_environment_for_assignment(
    pool: &sqlx::PgPool,
    project_id: i64,
    environment_id: i64,
) -> Result<ProjectEnvironmentAssignmentContext, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT code, name, status
        FROM project_environments
        WHERE project_id = $1
          AND id = $2
        LIMIT 1
        "#,
    )
    .bind(project_id)
    .bind(environment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        ApiError::internal_with(error, "failed to load project environment for assignment")
    })?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "project_environment_not_found",
            "project environment not found",
        )
    })?;

    Ok(ProjectEnvironmentAssignmentContext {
        code: row.get("code"),
        name: row.get("name"),
        status: row.get("status"),
    })
}

async fn load_template_context(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<TemplateDeploymentContext, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT project_id, is_template, is_archived, to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to load template context"))?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(TemplateDeploymentContext {
        project_id: row.get("project_id"),
        is_template: row.get("is_template"),
        is_archived: row.get("is_archived"),
        deleted_at: row.get("deleted_at"),
    })
}

async fn load_deployment_context(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<DeploymentLifecycleContext, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT
            project_id,
            btrim(deployment_uid::text) AS deployment_uid,
            is_template,
            status,
            is_archived,
            to_char(deleted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS deleted_at
        FROM deployment_instances
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to load deployment context"))?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(DeploymentLifecycleContext {
        project_id: row.get("project_id"),
        deployment_uid: row.get("deployment_uid"),
        is_template: row.get("is_template"),
        status: row.get("status"),
        is_archived: row.get("is_archived"),
        deleted_at: row.get("deleted_at"),
    })
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
    .map_err(|error| {
        ApiError::internal_with(error, "failed to upsert default deployment credential")
    })?;

    Ok(row.get("credential_name"))
}

async fn upsert_default_deployment_credential_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| {
        ApiError::internal_with(
            error,
            "failed to upsert default deployment credential in tx",
        )
    })?;

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
            pe.code AS environment_code,
            di.deployment_key,
            di.name AS deployment_name
        FROM deployment_instances di
        JOIN projects p ON p.id = di.project_id
        JOIN project_environments pe
          ON pe.project_id = di.project_id
         AND pe.id = di.environment_id
        WHERE di.id = $1
        LIMIT 1
        "#,
    )
    .bind(deployment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| ApiError::internal_with(error, "failed to load preview context"))?
    .ok_or_else(|| {
        ApiError::not_found_with(
            "deployment_instance_not_found",
            "deployment instance not found",
        )
    })?;

    Ok(PreviewDeploymentContext {
        project_id: row.get("project_id"),
        project_code: row.get("project_code"),
        environment_code: row.get("environment_code"),
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
    .map_err(|error| ApiError::internal_with(error, "failed to clone drafts from template"))?;

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

fn validate_page(page: Option<i64>, page_size: Option<i64>) -> Result<(i64, i64), ApiError> {
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    if page < 1 {
        return Err(ApiError::bad_request(
            "invalid_request",
            "page must be greater than or equal to 1",
        ));
    }
    if !(1..=100).contains(&page_size) {
        return Err(ApiError::bad_request(
            "invalid_request",
            "page_size must be between 1 and 100",
        ));
    }

    Ok((page, page_size))
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
        "environment_id" => "missing required body field: environment_id",
        "deployment_key" => "missing required body field: deployment_key",
        "name" => "missing required body field: name",
        "clone_source" => "missing required body field: clone_source",
        _ => "missing required body field",
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

#[cfg(test)]
mod tests {
    use super::{
        ArchiveDeploymentInstanceRequest, CloneDeploymentInstanceRequest,
        CreateDeploymentInstanceRequest, DeleteDeploymentInstanceRequest,
        DeploymentLifecycleContext, DeploymentVisibilityFilter, UpdateDeploymentInstanceRequest,
        ensure_not_archived_or_deleted, invalid_body_message, normalize_optional,
        parse_visibility_filter, required, required_i64, validate_clone_source, validate_page,
    };

    fn error_code(error: crate::error::ApiError) -> String {
        error.into_body().code
    }

    fn live_context() -> DeploymentLifecycleContext {
        DeploymentLifecycleContext {
            project_id: 7,
            deployment_uid: "11111111-1111-1111-1111-111111111111".to_owned(),
            is_template: false,
            status: "inactive".to_owned(),
            is_archived: false,
            deleted_at: None,
        }
    }

    #[test]
    fn visibility_filter_defaults_and_accepts_known_values() {
        assert_eq!(
            parse_visibility_filter(None),
            Ok(DeploymentVisibilityFilter::Current)
        );
        assert_eq!(
            parse_visibility_filter(Some("   ")),
            Ok(DeploymentVisibilityFilter::Current)
        );
        assert_eq!(
            parse_visibility_filter(Some("current")),
            Ok(DeploymentVisibilityFilter::Current)
        );
        assert_eq!(
            parse_visibility_filter(Some("archived")),
            Ok(DeploymentVisibilityFilter::Archived)
        );
        assert_eq!(
            parse_visibility_filter(Some("all")),
            Ok(DeploymentVisibilityFilter::All)
        );
        assert_eq!(DeploymentVisibilityFilter::Archived.as_str(), "archived");
    }

    #[test]
    fn visibility_filter_rejects_unknown_values() {
        let result = parse_visibility_filter(Some("deleted"));

        let error = result.map_err(error_code).err();
        assert_eq!(error, Some("invalid_request".to_owned()));
    }

    #[test]
    fn archive_and_delete_requests_trim_optional_reasons() {
        let archive = ArchiveDeploymentInstanceRequest {
            reason: Some("  seasonal cleanup  ".to_owned()),
        }
        .validate();
        let delete = DeleteDeploymentInstanceRequest {
            reason: Some("  ".to_owned()),
        }
        .validate();

        assert_eq!(archive.reason, Some("seasonal cleanup".to_owned()));
        assert_eq!(delete.reason, None);
    }

    #[test]
    fn ensure_not_archived_or_deleted_accepts_live_context() {
        assert_eq!(ensure_not_archived_or_deleted(&live_context()), Ok(()));
    }

    #[test]
    fn ensure_not_archived_or_deleted_rejects_archived_context() {
        let mut context = live_context();
        context.is_archived = true;

        assert_eq!(
            ensure_not_archived_or_deleted(&context).map_err(error_code),
            Err("deployment_instance_archived".to_owned())
        );
    }

    #[test]
    fn ensure_not_archived_or_deleted_rejects_deleted_context() {
        let mut context = live_context();
        context.deleted_at = Some("2026-04-21T00:00:00Z".to_owned());

        assert_eq!(
            ensure_not_archived_or_deleted(&context).map_err(error_code),
            Err("deployment_instance_deleted".to_owned())
        );
    }

    #[test]
    fn create_request_validation_trims_fields_and_defaults_template_flag() {
        let validated = CreateDeploymentInstanceRequest {
            project_id: Some(1),
            environment_id: Some(2),
            deployment_key: Some("  store-001  ".to_owned()),
            name: Some("  Store 001  ".to_owned()),
            description: Some("  Hangzhou  ".to_owned()),
            is_template: None,
        }
        .validate();

        assert_eq!(
            validated.map(|value| value.deployment_key),
            Ok("store-001".to_owned())
        );
    }

    #[test]
    fn create_request_validation_rejects_missing_required_fields() {
        let result = CreateDeploymentInstanceRequest {
            project_id: None,
            environment_id: Some(2),
            deployment_key: Some("store-001".to_owned()),
            name: Some("Store 001".to_owned()),
            description: None,
            is_template: Some(false),
        }
        .validate();

        let error = result.map_err(error_code).err();
        assert_eq!(error, Some("invalid_request".to_owned()));
    }

    #[test]
    fn update_request_validation_trims_description() {
        let validated = UpdateDeploymentInstanceRequest {
            environment_id: Some(2),
            deployment_key: Some("  store-002  ".to_owned()),
            name: Some("  Store 002  ".to_owned()),
            description: Some("  ".to_owned()),
        }
        .validate();

        assert_eq!(validated.map(|value| value.description), Ok(None));
    }

    #[test]
    fn clone_request_validation_requires_draft_source() {
        let validated = CloneDeploymentInstanceRequest {
            deployment_key: Some(" store-003 ".to_owned()),
            name: Some(" Store 003 ".to_owned()),
            environment_id: Some(2),
            description: None,
            clone_source: Some("draft".to_owned()),
        }
        .validate();

        assert_eq!(
            validated.map(|value| value.name),
            Ok("Store 003".to_owned())
        );

        let invalid = CloneDeploymentInstanceRequest {
            deployment_key: Some("store-004".to_owned()),
            name: Some("Store 004".to_owned()),
            environment_id: Some(2),
            description: None,
            clone_source: Some("latest_release".to_owned()),
        }
        .validate();

        let error = invalid.map_err(error_code).err();
        assert_eq!(error, Some("invalid_request".to_owned()));
    }

    #[test]
    fn pagination_validation_accepts_defaults_and_rejects_out_of_range_values() {
        assert_eq!(validate_page(None, None), Ok((1, 20)));
        assert_eq!(validate_page(Some(2), Some(100)), Ok((2, 100)));
        assert_eq!(
            validate_page(Some(0), Some(20)).map_err(error_code),
            Err("invalid_request".to_owned())
        );
        assert_eq!(
            validate_page(Some(1), Some(101)).map_err(error_code),
            Err("invalid_request".to_owned())
        );
    }

    #[test]
    fn scalar_validation_helpers_normalize_and_report_missing_fields() {
        assert_eq!(
            normalize_optional(Some("  store  ".to_owned())),
            Some("store".to_owned())
        );
        assert_eq!(normalize_optional(Some("  ".to_owned())), None);
        assert_eq!(
            required(Some("  value  ".to_owned()), "name"),
            Ok("value".to_owned())
        );
        assert_eq!(
            required(None, "name").map_err(error_code),
            Err("invalid_request".to_owned())
        );
        assert_eq!(required_i64(Some(42), "project_id"), Ok(42));
        assert_eq!(
            required_i64(None, "project_id").map_err(error_code),
            Err("invalid_request".to_owned())
        );
        assert_eq!(
            invalid_body_message("clone_source"),
            "missing required body field: clone_source"
        );
    }

    #[test]
    fn clone_source_validation_rejects_blank_or_unknown_values() {
        assert_eq!(
            validate_clone_source(Some(" draft ".to_owned())),
            Ok("draft".to_owned())
        );
        assert_eq!(
            validate_clone_source(Some(" ".to_owned())).map_err(error_code),
            Err("invalid_request".to_owned())
        );
        assert_eq!(
            validate_clone_source(Some("release".to_owned())).map_err(error_code),
            Err("invalid_request".to_owned())
        );
    }
}
