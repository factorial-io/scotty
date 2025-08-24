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
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Permission> {
        match s.to_lowercase().as_str() {
            "view" => Some(Permission::View),
            "manage" => Some(Permission::Manage),
            "shell" => Some(Permission::Shell),
            "logs" => Some(Permission::Logs),
            "create" => Some(Permission::Create),
            "destroy" => Some(Permission::Destroy),
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
    pub groups: HashMap<String, GroupConfig>,
    pub roles: HashMap<String, RoleConfig>,
    pub assignments: HashMap<String, Vec<Assignment>>,
    #[serde(default)]
    pub apps: HashMap<String, Vec<String>>,
}

/// Configuration structure for saving (excludes dynamically managed apps)
#[derive(Debug, Clone, Serialize)]
pub struct AuthConfigForSave {
    pub groups: HashMap<String, GroupConfig>,
    pub roles: HashMap<String, RoleConfig>,
    pub assignments: HashMap<String, Vec<Assignment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    #[serde(with = "permission_serde")]
    pub permissions: Vec<PermissionOrWildcard>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub role: String,
    pub groups: Vec<String>,
}

/// Custom serde module for permission serialization
pub mod permission_serde {
    use super::{Permission, PermissionOrWildcard};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(perms: &Vec<PermissionOrWildcard>, serializer: S) -> Result<S::Ok, S::Error>
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