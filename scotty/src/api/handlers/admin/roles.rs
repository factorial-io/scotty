use crate::api::basic_auth::CurrentUser;
use crate::{
    api::error::AppError, app_state::SharedAppState, 
    services::authorization::{Permission, types::PermissionOrWildcard},
};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct RoleInfo {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct RolesListResponse {
    pub roles: Vec<RoleInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct CreateRoleResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/roles",
    responses(
        (status = 200, response = inline(RolesListResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_roles_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!("Admin listing roles for user: {}", user.email);

    // Get all roles from authorization service
    let roles = auth_service.list_roles().await;

    let roles_info: Vec<RoleInfo> = roles
        .into_iter()
        .map(|(name, config)| {
            let permissions: Vec<String> = config
                .permissions
                .into_iter()
                .map(|p| match p {
                    PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                    PermissionOrWildcard::Wildcard => "*".to_string(),
                })
                .collect();

            RoleInfo {
                name,
                description: config.description,
                permissions,
            }
        })
        .collect();

    let response = RolesListResponse { roles: roles_info };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/roles",
    request_body = CreateRoleRequest,
    responses(
        (status = 200, response = inline(CreateRoleResponse)),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminWrite required"),
        (status = 409, description = "Role already exists"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_role_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<CreateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!(
        "Admin creating role '{}' for user: {}",
        request.name, user.email
    );

    // Validate input
    if request.name.trim().is_empty() {
        return Ok(Json(CreateRoleResponse {
            success: false,
            message: "Role name cannot be empty".to_string(),
        }));
    }

    if request.description.trim().is_empty() {
        return Ok(Json(CreateRoleResponse {
            success: false,
            message: "Role description cannot be empty".to_string(),
        }));
    }

    if request.permissions.is_empty() {
        return Ok(Json(CreateRoleResponse {
            success: false,
            message: "Role must have at least one permission".to_string(),
        }));
    }

    // Parse and validate permissions
    let mut parsed_permissions = Vec::new();
    for perm_str in &request.permissions {
        if perm_str == "*" {
            parsed_permissions.push(PermissionOrWildcard::Wildcard);
        } else if let Some(perm) = Permission::from_str(perm_str) {
            parsed_permissions.push(PermissionOrWildcard::Permission(perm));
        } else {
            return Ok(Json(CreateRoleResponse {
                success: false,
                message: format!("Invalid permission: '{}'", perm_str),
            }));
        }
    }

    // Check if role already exists
    let existing_roles = auth_service.list_roles().await;
    if existing_roles.iter().any(|(name, _)| name == &request.name) {
        return Ok(Json(CreateRoleResponse {
            success: false,
            message: format!("Role '{}' already exists", request.name),
        }));
    }

    // Create the role
    match auth_service
        .create_role(&request.name, parsed_permissions, &request.description)
        .await
    {
        Ok(_) => {
            info!("Successfully created role '{}'", request.name);
            Ok(Json(CreateRoleResponse {
                success: true,
                message: format!("Role '{}' created successfully", request.name),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to create role '{}': {}", request.name, e);
            Ok(Json(CreateRoleResponse {
                success: false,
                message: format!("Failed to create role: {}", e),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::AuthorizationService;
    use tempfile::tempdir;

    async fn create_test_service() -> (AuthorizationService, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_dir = temp_dir.path().to_str().unwrap();

        // Create model.conf
        let model_content = r#"[request_definition]
r = sub, app, act

[policy_definition]
p = sub, scope, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && g2(r.app, p.scope) && r.act == p.act"#;

        std::fs::write(
            format!("{}/model.conf", config_dir),
            model_content,
        ).expect("Failed to write model.conf");

        // Create empty policy.yaml
        let policy_content = r#"scopes:
  default:
    description: "Default scope"
    created_at: "2023-01-01T00:00:00Z"
roles:
  admin:
    description: "Admin role"
    permissions: ["*"]
assignments: {}
apps: {}"#;

        std::fs::write(
            format!("{}/policy.yaml", config_dir),
            policy_content,
        ).expect("Failed to write policy.yaml");

        let service = AuthorizationService::new(config_dir).await.expect("Failed to create service");
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_list_roles_with_test_service() {
        let (auth_service, _temp_dir) = create_test_service().await;

        let roles = auth_service.list_roles().await;

        // Test service should have at least the admin role
        assert!(!roles.is_empty());
        assert!(roles.iter().any(|(name, _)| name == "admin"));
    }

    #[tokio::test]
    async fn test_create_role_validation() {
        let (auth_service, _temp_dir) = create_test_service().await;

        // Test creating a valid role
        let permissions = vec![PermissionOrWildcard::Permission(Permission::View)];
        let result = auth_service
            .create_role("test-role", permissions, "Test role")
            .await;
        assert!(result.is_ok());

        // Verify role was created
        let roles = auth_service.list_roles().await;
        assert!(roles.iter().any(|(name, _)| name == "test-role"));
    }

    #[test]
    fn test_permission_parsing() {
        assert_eq!(Permission::from_str("view"), Some(Permission::View));
        assert_eq!(Permission::from_str("admin_read"), Some(Permission::AdminRead));
        assert_eq!(Permission::from_str("admin_write"), Some(Permission::AdminWrite));
        assert_eq!(Permission::from_str("invalid"), None);
    }
}