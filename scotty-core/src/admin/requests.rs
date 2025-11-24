use serde::{Deserialize, Serialize};

/// Request to create a new scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateScopeRequest {
    /// Name of the scope
    pub name: String,
    /// Description of the scope
    pub description: String,
}

/// Request to create a new role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateRoleRequest {
    /// Name of the role
    pub name: String,
    /// Description of the role
    pub description: String,
    /// Permissions for the role (comma-separated). Use '*' for wildcard
    #[cfg_attr(feature = "clap", arg(long, value_delimiter = ','))]
    pub permissions: Vec<String>,
}

/// Request to create a user assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateAssignmentRequest {
    /// User identifier (e.g., identifier:admin, user@example.com)
    pub user_id: String,
    /// Role name to assign
    pub role: String,
    /// Scopes to assign the role to (comma-separated)
    #[cfg_attr(feature = "clap", arg(long, value_delimiter = ','))]
    pub scopes: Vec<String>,
}

/// Request to remove a user assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RemoveAssignmentRequest {
    /// User identifier (e.g., identifier:admin, user@example.com)
    pub user_id: String,
    /// Role name to remove
    pub role: String,
    /// Scopes to remove the role from (comma-separated)
    #[cfg_attr(feature = "clap", arg(long, value_delimiter = ','))]
    pub scopes: Vec<String>,
}

/// Request to test permission for a user on an app
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct TestPermissionRequest {
    /// User identifier to test (defaults to current user if not specified)
    #[cfg_attr(feature = "clap", arg(long, short = 'u'))]
    #[cfg_attr(feature = "utoipa", schema(example = "identifier:admin"))]
    pub user_id: Option<String>, // None means test current user
    /// App name to test permission on
    pub app_name: String,
    /// Permission to test (e.g., view, manage, shell, logs, create, destroy, admin_read, admin_write)
    pub permission: String,
}

/// Request to get permissions for a specific user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Parser))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct GetUserPermissionsRequest {
    /// User identifier to get permissions for
    pub user_id: String,
}
