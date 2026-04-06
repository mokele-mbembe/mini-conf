use crate::{error::ErrorResponse, state::AppState};
use axum::Router;
use schema::{
    auth::{AuthSessionResponse, AuthUser},
    health::HealthzResponse,
    open::{
        ConfigBundleResponse, DeploymentSyncResponse, ReleaseContentResponse, ResolveConfigResponse,
    },
    project::{ProjectListResponse, ProjectSummary},
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
        crate::http::api::projects::list_projects,
        crate::http::api::projects::create_project,
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
            HealthzResponse,
            ResolveConfigResponse,
            ReleaseContentResponse,
            ConfigBundleResponse,
            DeploymentSyncResponse,
            ProjectSummary,
            ProjectListResponse,
            LoginRequestBody,
            CreateProjectRequestBody,
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
pub struct CreateProjectRequestBody {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ResolveConfigParams {
    pub project: String,
    pub environment: String,
    pub deployment_key: String,
    pub config: String,
    pub process_key: Option<String>,
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
    pub process_key: Option<String>,
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
    pub process_key: String,
    #[schema(value_type = Object, nullable = true)]
    pub metadata: Option<serde_json::Value>,
    pub reported_at: Option<String>,
}

pub fn document() -> OpenApiDocument {
    OPENAPI.get_or_init(ApiDoc::openapi).clone()
}

pub fn router() -> Router<AppState> {
    Router::new().merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", document()))
}

pub fn export_to(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let openapi = document();
    let json = serde_json::to_string_pretty(&openapi)?;

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
        assert!(paths.contains_key("/api/projects"));
        assert!(paths.contains_key("/api/open/configs/resolve"));
        assert!(paths.contains_key("/api/open/releases/{revision}"));
        assert!(paths.contains_key("/api/open/deployments/{deployment_key}/config-bundle"));
        assert!(paths.contains_key("/api/open/deployment-sync-records"));
        assert!(paths.contains_key("/api/open/heartbeats"));

        let components = openapi.components.expect("components should exist");
        assert!(components.security_schemes.contains_key("bearer_auth"));
        assert!(components.security_schemes.contains_key("session_auth"));
    }
}
