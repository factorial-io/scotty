use serde::{Deserialize, Serialize};

/// Available permissions/actions for authorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
    /// Get all available permissions in display order
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::View,
            Permission::Manage,
            Permission::Create,
            Permission::Destroy,
            Permission::Shell,
            Permission::Logs,
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
