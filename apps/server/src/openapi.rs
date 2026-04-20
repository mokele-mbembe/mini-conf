use crate::{error::ErrorResponse, state::AppState};
use axum::Router;
use schema::{
    audit::{
        AuditLogListResponse, AuditLogSummary, DeploymentHeartbeatListResponse,
        DeploymentHeartbeatSummary, DeploymentSyncRecordListResponse, DeploymentSyncRecordSummary,
    },
    auth::{AuthSessionResponse, AuthUser},
    clone_source::{CloneSourceAvailability, CloneSourceListResponse, CloneSourceSummary},
    config_file::{ConfigFileListResponse, ConfigFileSummary},
    deployment_instance::{
        DeploymentBundlePreviewResponse, DeploymentInstanceListResponse, DeploymentInstanceSummary,
        DeploymentTokenResetResponse,
    },
    draft::{DraftCloneResponse, DraftResponse},
    health::HealthzResponse,
    open::{
        ConfigBundleResponse, DeploymentSyncResponse, ReleaseContentResponse, ResolveConfigResponse,
    },
    project::{ProjectListResponse, ProjectSummary},
    project_environment::{ProjectEnvironmentListResponse, ProjectEnvironmentSummary},
    project_member::{ProjectMemberListResponse, ProjectMemberSummary},
    release::{
        ReleaseDetailResponse, ReleaseDiffResponse, ReleaseDiffSummary, ReleaseListResponse,
        ReleaseSummary,
    },
    saved_version::{
        SavedVersionDetail, SavedVersionDetailResponse, SavedVersionListResponse,
        SavedVersionRestoreResponse, SavedVersionSummary,
    },
};
use std::{fs, path::Path, sync::OnceLock};
use utoipa::{
    IntoParams, Modify, OpenApi, ToSchema,
    openapi::{
        Components, OpenApi as OpenApiDocument,
        security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme},
    },
};
use utoipa_swagger_ui::SwaggerUi;

static OPENAPI: OnceLock<OpenApiDocument> = OnceLock::new();

#[derive(OpenApi)]
#[openapi(
    info(
        title = "mini-conf API",
        description = "HTTP-first deployment configuration APIs for mini-conf",
        version = "0.1.0"
    ),
    paths(
        crate::http::api::health::get_healthz,
        crate::http::api::auth::login,
        crate::http::api::auth::logout,
        crate::http::api::auth::me,
        crate::http::api::audit_logs::list_audit_logs,
        crate::http::api::clone_sources::list_clone_sources,
        crate::http::api::config_files::list_config_files,
        crate::http::api::config_files::create_config_file,
        crate::http::api::config_files::get_config_file,
        crate::http::api::config_files::update_config_file,
        crate::http::api::deployment_instances::list_deployment_instances,
        crate::http::api::deployment_instances::create_deployment_instance,
        crate::http::api::deployment_instances::get_deployment_instance,
        crate::http::api::deployment_instances::update_deployment_instance,
        crate::http::api::deployment_instances::clone_deployment_instance,
        crate::http::api::deployment_instances::preview_deployment_bundle,
        crate::http::api::deployment_instances::reset_deployment_token,
        crate::http::api::deployment_instances::activate_deployment_instance,
        crate::http::api::deployment_instances::deactivate_deployment_instance,
        crate::http::api::deployment_heartbeats::list_deployment_heartbeats,
        crate::http::api::deployment_sync_records::list_deployment_sync_records,
        crate::http::api::drafts::get_draft,
        crate::http::api::drafts::put_draft,
        crate::http::api::drafts::delete_draft,
        crate::http::api::drafts::clone_draft,
        crate::http::api::project_members::list_project_members,
        crate::http::api::project_members::create_project_member,
        crate::http::api::project_members::update_project_member,
        crate::http::api::project_members::delete_project_member,
        crate::http::api::project_environments::list_project_environments,
        crate::http::api::project_environments::create_project_environment,
        crate::http::api::project_environments::get_project_environment,
        crate::http::api::project_environments::update_project_environment,
        crate::http::api::project_environments::delete_project_environment,
        crate::http::api::projects::list_projects,
        crate::http::api::projects::create_project,
        crate::http::api::projects::get_project,
        crate::http::api::projects::update_project,
        crate::http::api::releases::list_releases,
        crate::http::api::releases::publish_release,
        crate::http::api::releases::get_release_detail,
        crate::http::api::releases::get_release_diff,
        crate::http::api::saved_versions::list_saved_versions,
        crate::http::api::saved_versions::get_saved_version,
        crate::http::api::saved_versions::update_saved_version,
        crate::http::api::saved_versions::restore_saved_version,
        crate::http::api::saved_versions::delete_saved_version,
        crate::http::api::open::configs::resolve_config,
        crate::http::api::open::releases::get_release,
        crate::http::api::open::deployments::get_config_bundle,
        crate::http::api::open::sync_records::create_sync_record,
        crate::http::api::open::heartbeats::report_heartbeat,
    ),
    components(
        schemas(
            ErrorResponse,
            AuthSessionResponse,
            AuthUser,
            AuditLogSummary,
            AuditLogListResponse,
            CloneSourceAvailability,
            CloneSourceSummary,
            CloneSourceListResponse,
            DeploymentHeartbeatSummary,
            DeploymentHeartbeatListResponse,
            ConfigFileSummary,
            ConfigFileListResponse,
            DeploymentSyncRecordSummary,
            DeploymentSyncRecordListResponse,
            DraftResponse,
            DraftCloneResponse,
            DeploymentInstanceSummary,
            DeploymentInstanceListResponse,
            DeploymentBundlePreviewResponse,
            DeploymentTokenResetResponse,
            HealthzResponse,
            ProjectMemberSummary,
            ProjectMemberListResponse,
            ProjectEnvironmentSummary,
            ProjectEnvironmentListResponse,
            ResolveConfigResponse,
            ReleaseContentResponse,
            ReleaseSummary,
            ReleaseDiffSummary,
            ConfigBundleResponse,
            DeploymentSyncResponse,
            ProjectSummary,
            ProjectListResponse,
            ReleaseListResponse,
            ReleaseDetailResponse,
            ReleaseDiffResponse,
            SavedVersionSummary,
            SavedVersionListResponse,
            SavedVersionDetail,
            SavedVersionDetailResponse,
            SavedVersionRestoreResponse,
            LoginRequestBody,
            CreateConfigFileRequestBody,
            CreateDeploymentInstanceRequestBody,
            CloneDeploymentInstanceRequestBody,
            UpdateDraftRequestBody,
            CloneDraftRequestBody,
            UpdateDeploymentInstanceRequestBody,
            UpdateConfigFileRequestBody,
            CreateProjectMemberRequestBody,
            CreateProjectEnvironmentRequestBody,
            CreateProjectRequestBody,
            UpdateProjectEnvironmentRequestBody,
            UpdateProjectRequestBody,
            UpdateProjectMemberRequestBody,
            PublishReleaseRequestBody,
            ListAuditLogsParams,
            ListCloneSourcesParams,
            ListConfigFilesParams,
            ListDeploymentHeartbeatsParams,
            ListDeploymentSyncRecordsParams,
            ListDeploymentInstancesParams,
            ListProjectsParams,
            ListReleasesParams,
            ListSavedVersionsParams,
            UpdateSavedVersionRequestBody,
            RestoreSavedVersionRequestBody,
            ResolveConfigParams,
            ConfigBundleParams,
            DeploymentSyncRecordRequestBody,
            HeartbeatRequestBody,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "system", description = "System and health endpoints"),
        (name = "auth", description = "Management authentication APIs"),
        (name = "admin", description = "Management APIs"),
        (name = "open", description = "Deployment instance access APIs")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut OpenApiDocument) {
        let components = openapi.components.get_or_insert_with(Components::new);

        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("opaque")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "session_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("mini_conf_session"))),
        );
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct LoginRequestBody {
    pub username: String,
    pub password: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateConfigFileRequestBody {
    pub project_id: i64,
    pub code: String,
    pub name: String,
    pub is_required: Option<bool>,
    #[schema(example = "toml")]
    pub format: String,
    pub sensitivity: Option<String>,
    pub secret_paths: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateDeploymentInstanceRequestBody {
    pub project_id: i64,
    pub environment_id: i64,
    pub deployment_key: String,
    pub name: String,
    pub description: Option<String>,
    pub is_template: Option<bool>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CloneDeploymentInstanceRequestBody {
    pub deployment_key: String,
    pub name: String,
    pub environment_id: i64,
    pub description: Option<String>,
    pub clone_source: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateDeploymentInstanceRequestBody {
    pub environment_id: i64,
    pub deployment_key: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateProjectEnvironmentRequestBody {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateProjectEnvironmentRequestBody {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateDraftRequestBody {
    pub content: String,
    #[schema(example = "toml")]
    pub format: String,
    pub base_version: Option<i64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CloneDraftRequestBody {
    pub source_deployment_instance_id: i64,
    pub source_kind: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateConfigFileRequestBody {
    pub project_id: i64,
    pub code: String,
    pub name: String,
    pub is_required: Option<bool>,
    #[schema(example = "toml")]
    pub format: String,
    pub sensitivity: Option<String>,
    pub secret_paths: Option<Vec<String>>,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct CreateProjectRequestBody {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateProjectRequestBody {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
#[schema(example = create_project_member_request_example)]
pub struct CreateProjectMemberRequestBody {
    pub username: String,
    pub role: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
#[schema(example = update_project_member_request_example)]
pub struct UpdateProjectMemberRequestBody {
    pub role: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct PublishReleaseRequestBody {
    pub project_id: i64,
    pub deployment_instance_id: i64,
    pub config_file_id: i64,
    pub change_summary: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListProjectsParams {
    pub status: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListConfigFilesParams {
    pub project_id: Option<i64>,
    pub status: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListDeploymentInstancesParams {
    pub project_id: Option<i64>,
    pub environment_id: Option<i64>,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub is_template: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListDeploymentHeartbeatsParams {
    pub project_id: Option<i64>,
    pub deployment_instance_id: Option<i64>,
    pub config_file_id: Option<i64>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListDeploymentSyncRecordsParams {
    pub project_id: Option<i64>,
    pub deployment_instance_id: Option<i64>,
    pub config_file_id: Option<i64>,
    pub action: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListReleasesParams {
    pub project_id: Option<i64>,
    pub deployment_instance_id: Option<i64>,
    pub config_file_id: Option<i64>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListSavedVersionsParams {
    pub deployment_instance_id: Option<i64>,
    pub config_file_id: Option<i64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct UpdateSavedVersionRequestBody {
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct RestoreSavedVersionRequestBody {
    pub base_version: Option<i64>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListCloneSourcesParams {
    pub project_id: i64,
    pub target_deployment_id: i64,
    pub config_file_id: i64,
    pub keyword: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<i64>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListAuditLogsParams {
    pub project_id: Option<i64>,
    pub user_id: Option<i64>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ResolveConfigParams {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
    pub config: String,
    pub current_revision: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ConfigBundleParams {
    pub project: String,
    pub environment: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct DeploymentSyncRecordRequestBody {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
    pub config: String,
    pub action: String,
    pub revision: Option<String>,
    pub status: String,
    pub message: Option<String>,
    #[schema(value_type = Object, nullable = true)]
    pub detail: Option<serde_json::Value>,
    pub reported_at: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct HeartbeatRequestBody {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
    pub config: String,
    #[schema(value_type = Object, nullable = true)]
    pub metadata: Option<serde_json::Value>,
    pub reported_at: Option<String>,
}

fn create_project_member_request_example() -> serde_json::Value {
    serde_json::json!({
        "username": "alice",
        "role": "viewer"
    })
}

fn update_project_member_request_example() -> serde_json::Value {
    serde_json::json!({
        "role": "admin"
    })
}

pub fn document() -> OpenApiDocument {
    OPENAPI.get_or_init(ApiDoc::openapi).clone()
}

pub fn router() -> Router<AppState> {
    Router::new().merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", document()))
}

pub fn export_to(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let openapi = document();
    let mut json = serde_json::to_string_pretty(&openapi)?;
    json.push('\n');

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::document;

    #[test]
    fn document_includes_core_paths_and_bearer_security() {
        let openapi = document();
        let paths = openapi.paths.paths;

        assert!(paths.contains_key("/api/healthz"));
        assert!(paths.contains_key("/api/auth/login"));
        assert!(paths.contains_key("/api/auth/logout"));
        assert!(paths.contains_key("/api/auth/me"));
        assert!(paths.contains_key("/api/config-files"));
        assert!(paths.contains_key("/api/config-files/{id}"));
        assert!(paths.contains_key("/api/deployment-instances"));
        assert!(paths.contains_key("/api/deployment-instances/{id}"));
        assert!(paths.contains_key("/api/deployment-instances/{id}/clone"));
        assert!(paths.contains_key("/api/deployment-instances/{id}/preview-bundle"));
        assert!(paths.contains_key("/api/deployment-instances/{id}/token/reset"));
        assert!(paths.contains_key("/api/deployment-instances/{id}/activate"));
        assert!(paths.contains_key("/api/deployment-instances/{id}/deactivate"));
        assert!(paths.contains_key("/api/deployment-heartbeats"));
        assert!(paths.contains_key("/api/drafts/{deployment_id}/{config_file_id}"));
        assert!(paths.contains_key("/api/drafts/{target_deployment_id}/{config_file_id}/clone"));
        assert!(paths.contains_key("/api/projects"));
        assert!(paths.contains_key("/api/projects/{id}"));
        assert!(paths.contains_key("/api/releases"));
        assert!(paths.contains_key("/api/releases/publish"));
        assert!(paths.contains_key("/api/releases/{id}"));
        assert!(paths.contains_key("/api/releases/{id}/diff"));
        assert!(paths.contains_key("/api/open/configs/resolve"));
        assert!(paths.contains_key("/api/open/releases/{revision}"));
        assert!(paths.contains_key("/api/open/deployments/{deployment_key}/config-bundle"));
        assert!(paths.contains_key("/api/open/deployment-sync-records"));
        assert!(paths.contains_key("/api/draft-saved-versions"));
        assert!(paths.contains_key("/api/draft-saved-versions/{id}"));
        assert!(paths.contains_key("/api/draft-saved-versions/{id}/restore"));
        assert!(paths.contains_key("/api/open/heartbeats"));

        let components = openapi.components.expect("components should exist");
        assert!(components.security_schemes.contains_key("bearer_auth"));
        assert!(components.security_schemes.contains_key("session_auth"));
    }
}
