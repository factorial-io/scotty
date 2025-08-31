use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about a scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct ScopeInfo {
    pub name: String,
    pub description: String,
    pub created_at: String,
}

/// Response for listing scopes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct ScopesListResponse {
    pub scopes: Vec<ScopeInfo>,
}

/// Information about a role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct RoleInfo {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

/// Response for listing roles
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct RolesListResponse {
    pub roles: Vec<RoleInfo>,
}

/// Information about a user's assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Assignment {
    pub role: String,
    pub scopes: Vec<String>,
}

/// Information about assignments for a specific user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct AssignmentInfo {
    pub user_id: String,
    pub assignments: Vec<Assignment>,
}

/// Response for listing assignments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct AssignmentsListResponse {
    pub assignments: Vec<AssignmentInfo>,
}

/// Generic success response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Response for permission test
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct TestPermissionResponse {
    pub user_id: String,
    pub app_name: String,
    pub permission: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

/// Response for user permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct UserPermissionsResponse {
    pub user_id: String,
    pub permissions: HashMap<String, Vec<String>>,
}

/// Response for available permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema, utoipa::ToResponse))]
pub struct AvailablePermissionsResponse {
    pub permissions: Vec<String>,
}

// Re-export common types for convenience
pub type CreateScopeResponse = SuccessResponse;
pub type CreateRoleResponse = SuccessResponse;
pub type CreateAssignmentResponse = SuccessResponse;
pub type RemoveAssignmentResponse = SuccessResponse;
