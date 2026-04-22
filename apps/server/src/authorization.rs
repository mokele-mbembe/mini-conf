use crate::{
    auth::{AuthenticatedUser, authenticate_admin_session},
    error::ApiError,
};
use axum::http::{HeaderMap, header};
use sqlx::{PgPool, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRole {
    Viewer,
    Editor,
    Admin,
}

impl ProjectRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Editor => "editor",
            Self::Admin => "admin",
        }
    }

    pub fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "viewer" => Ok(Self::Viewer),
            "editor" => Ok(Self::Editor),
            "admin" => Ok(Self::Admin),
            _ => Err(ApiError::bad_request(
                "invalid_request",
                "invalid project member role",
            )),
        }
    }

    pub const fn allows(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Admin => 3,
        }
    }
}

pub async fn authenticate_user(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<AuthenticatedUser, ApiError> {
    authenticate_admin_session(
        pool,
        headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok()),
    )
    .await
}

pub async fn require_platform_admin(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<AuthenticatedUser, ApiError> {
    let auth = authenticate_user(pool, headers).await?;

    if !auth.is_platform_admin {
        return Err(ApiError::forbidden(
            "platform_permission_denied",
            "Platform admin access is required",
        ));
    }

    Ok(auth)
}

pub async fn require_project_role(
    pool: &PgPool,
    user_id: i64,
    project_id: i64,
    required: ProjectRole,
    not_found_code: &'static str,
    not_found_message: &'static str,
) -> Result<ProjectRole, ApiError> {
    let Some(role) = load_project_role(pool, user_id, project_id).await? else {
        return Err(ApiError::not_found_with(not_found_code, not_found_message));
    };

    if !role.allows(required) {
        return Err(ApiError::forbidden(
            "project_permission_denied",
            "Project role does not allow this action",
        ));
    }

    Ok(role)
}

pub async fn load_project_role(
    pool: &PgPool,
    user_id: i64,
    project_id: i64,
) -> Result<Option<ProjectRole>, ApiError> {
    let row = sqlx::query(
        r#"
        SELECT role
        FROM project_members
        WHERE project_id = $1
          AND user_id = $2
        LIMIT 1
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal())?;

    row.map(|row| ProjectRole::parse(row.get::<String, _>("role").as_str()))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::ProjectRole;

    #[test]
    fn project_role_allows_matches_role_hierarchy() {
        for (actual, required, expected) in [
            (ProjectRole::Viewer, ProjectRole::Viewer, true),
            (ProjectRole::Viewer, ProjectRole::Editor, false),
            (ProjectRole::Viewer, ProjectRole::Admin, false),
            (ProjectRole::Editor, ProjectRole::Viewer, true),
            (ProjectRole::Editor, ProjectRole::Editor, true),
            (ProjectRole::Editor, ProjectRole::Admin, false),
            (ProjectRole::Admin, ProjectRole::Viewer, true),
            (ProjectRole::Admin, ProjectRole::Editor, true),
            (ProjectRole::Admin, ProjectRole::Admin, true),
        ] {
            assert_eq!(
                actual.allows(required),
                expected,
                "{actual:?} allowing {required:?} should be {expected}"
            );
        }
    }
}
