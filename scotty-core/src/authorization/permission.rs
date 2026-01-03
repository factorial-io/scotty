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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_permissions_exist() {
        // Verify all action permissions are in the all() list
        let all = Permission::all();
        assert!(all.contains(&Permission::ActionRead));
        assert!(all.contains(&Permission::ActionWrite));
        assert!(all.contains(&Permission::ActionManage));
        assert!(all.contains(&Permission::ActionApprove));
    }

    #[test]
    fn test_action_permissions_as_str() {
        assert_eq!(Permission::ActionRead.as_str(), "action_read");
        assert_eq!(Permission::ActionWrite.as_str(), "action_write");
        assert_eq!(Permission::ActionManage.as_str(), "action_manage");
        assert_eq!(Permission::ActionApprove.as_str(), "action_approve");
    }

    #[test]
    fn test_action_permissions_from_str() {
        assert_eq!(
            Permission::from_str("action_read"),
            Some(Permission::ActionRead)
        );
        assert_eq!(
            Permission::from_str("action_write"),
            Some(Permission::ActionWrite)
        );
        assert_eq!(
            Permission::from_str("action_manage"),
            Some(Permission::ActionManage)
        );
        assert_eq!(
            Permission::from_str("action_approve"),
            Some(Permission::ActionApprove)
        );
    }

    #[test]
    fn test_action_permissions_from_str_case_insensitive() {
        assert_eq!(
            Permission::from_str("ACTION_READ"),
            Some(Permission::ActionRead)
        );
        assert_eq!(
            Permission::from_str("Action_Write"),
            Some(Permission::ActionWrite)
        );
    }

    #[test]
    fn test_action_permissions_serialization() {
        let json = serde_json::to_string(&Permission::ActionRead).unwrap();
        assert_eq!(json, "\"action_read\"");

        let json = serde_json::to_string(&Permission::ActionWrite).unwrap();
        assert_eq!(json, "\"action_write\"");

        let json = serde_json::to_string(&Permission::ActionManage).unwrap();
        assert_eq!(json, "\"action_manage\"");

        let json = serde_json::to_string(&Permission::ActionApprove).unwrap();
        assert_eq!(json, "\"action_approve\"");
    }

    #[test]
    fn test_action_permissions_deserialization() {
        let perm: Permission = serde_json::from_str("\"action_read\"").unwrap();
        assert_eq!(perm, Permission::ActionRead);

        let perm: Permission = serde_json::from_str("\"action_write\"").unwrap();
        assert_eq!(perm, Permission::ActionWrite);

        let perm: Permission = serde_json::from_str("\"action_manage\"").unwrap();
        assert_eq!(perm, Permission::ActionManage);

        let perm: Permission = serde_json::from_str("\"action_approve\"").unwrap();
        assert_eq!(perm, Permission::ActionApprove);
    }

    #[test]
    fn test_admin_permissions_backward_compatibility() {
        // Test that old lowercase format still works via alias
        let perm: Permission = serde_json::from_str("\"adminread\"").unwrap();
        assert_eq!(perm, Permission::AdminRead);

        let perm: Permission = serde_json::from_str("\"adminwrite\"").unwrap();
        assert_eq!(perm, Permission::AdminWrite);

        // New snake_case format should also work
        let perm: Permission = serde_json::from_str("\"admin_read\"").unwrap();
        assert_eq!(perm, Permission::AdminRead);

        let perm: Permission = serde_json::from_str("\"admin_write\"").unwrap();
        assert_eq!(perm, Permission::AdminWrite);
    }

    #[test]
    fn test_roundtrip_all_permissions() {
        for perm in Permission::all() {
            let json = serde_json::to_string(&perm).unwrap();
            let parsed: Permission = serde_json::from_str(&json).unwrap();
            assert_eq!(perm, parsed);
        }
    }

    #[test]
    fn test_from_str_roundtrip() {
        for perm in Permission::all() {
            let s = perm.as_str();
            let parsed = Permission::from_str(s);
            assert_eq!(Some(perm), parsed);
        }
    }

    #[test]
    fn test_invalid_permission_returns_none() {
        assert_eq!(Permission::from_str("invalid"), None);
        assert_eq!(Permission::from_str(""), None);
        assert_eq!(Permission::from_str("action"), None);
    }
}
