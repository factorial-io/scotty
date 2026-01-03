use serde::{Deserialize, Serialize};

/// Available permissions/actions for authorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    View,
    Manage,
    Shell,
    Logs,
    Create,
    Destroy,
    /// Read-only admin access
    #[serde(alias = "adminread")]
    AdminRead,
    /// Write admin access
    #[serde(alias = "adminwrite")]
    AdminWrite,
    /// Permission to execute safe/read-only actions (no side effects)
    ActionRead,
    /// Permission to execute actions that modify state
    ActionWrite,
    /// Permission to create, list, and delete custom actions for apps in user's scope
    ActionManage,
    /// Permission to approve/reject pending custom actions (admin-level)
    ActionApprove,
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
            Permission::ActionRead,
            Permission::ActionWrite,
            Permission::ActionManage,
            Permission::ActionApprove,
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
            Permission::ActionRead => "action_read",
            Permission::ActionWrite => "action_write",
            Permission::ActionManage => "action_manage",
            Permission::ActionApprove => "action_approve",
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
            "action_read" => Some(Permission::ActionRead),
            "action_write" => Some(Permission::ActionWrite),
            "action_manage" => Some(Permission::ActionManage),
            "action_approve" => Some(Permission::ActionApprove),
            _ => None,
        }
    }
}
