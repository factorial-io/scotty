use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Available permissions/actions for authorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    View,
    Manage,
    Shell,
    Logs,
    Create,
    Destroy,
    AdminRead,
    AdminWrite,
}

impl Permission {
    /// Get all available permissions
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::View,
            Permission::Manage,
            Permission::Shell,
            Permission::Logs,
            Permission::Create,
            Permission::Destroy,
            Permission::AdminRead,
            Permission::AdminWrite,
        ]
    }

    /// Convert to string for Casbin policy
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::View => "view",
            Permission::Manage => "manage",
            Permission::Shell => "shell",
            Permission::Logs => "logs",
            Permission::Create => "create",
            Permission::Destroy => "destroy",
            Permission::AdminRead => "admin_read",
            Permission::AdminWrite => "admin_write",
        }
    }

    /// Parse from string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Permission> {
        match s.to_lowercase().as_str() {
            "view" => Some(Permission::View),
            "manage" => Some(Permission::Manage),
            "shell" => Some(Permission::Shell),
            "logs" => Some(Permission::Logs),
            "create" => Some(Permission::Create),
            "destroy" => Some(Permission::Destroy),
            "admin_read" => Some(Permission::AdminRead),
            "admin_write" => Some(Permission::AdminWrite),
            _ => None,
        }
    }
}

/// Represents either a specific permission or wildcard (*)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOrWildcard {
    Permission(Permission),
    Wildcard,
}

/// Authorization configuration loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub scopes: HashMap<String, ScopeConfig>,
    pub roles: HashMap<String, RoleConfig>,
    pub assignments: HashMap<String, Vec<Assignment>>,
    #[serde(default)]
    pub apps: HashMap<String, Vec<String>>, // Maps app_name -> scope_names
}

/// Configuration structure for saving (excludes dynamically managed apps)
#[derive(Debug, Clone, Serialize)]
pub struct AuthConfigForSave {
    pub scopes: HashMap<String, ScopeConfig>,
    pub roles: HashMap<String, RoleConfig>,
    pub assignments: HashMap<String, Vec<Assignment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeConfig {
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    #[serde(with = "permission_serde")]
    pub permissions: Vec<PermissionOrWildcard>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Assignment {
    pub role: String,
    pub scopes: Vec<String>,
}

/// Custom serde module for permission serialization
pub mod permission_serde {
    use super::{Permission, PermissionOrWildcard};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(perms: &[PermissionOrWildcard], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<String> = perms
            .iter()
            .map(|p| match p {
                PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                PermissionOrWildcard::Wildcard => "*".to_string(),
            })
            .collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<PermissionOrWildcard>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strings: Vec<String> = Vec::deserialize(deserializer)?;
        Ok(strings
            .into_iter()
            .map(|s| {
                if s == "*" {
                    PermissionOrWildcard::Wildcard
                } else if let Some(perm) = Permission::from_str(&s) {
                    PermissionOrWildcard::Permission(perm)
                } else {
                    // For backward compatibility, treat unknown strings as wildcard
                    PermissionOrWildcard::Wildcard
                }
            })
            .collect())
    }
}
