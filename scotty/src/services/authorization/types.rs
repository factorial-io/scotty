use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

// Re-export Permission from scotty-core
pub use scotty_core::authorization::Permission;

/// Represents either a specific permission or wildcard (*)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionOrWildcard {
    Permission(Permission),
    Wildcard,
    /// `create:<blueprint>`: create apps from that blueprint only, never from
    /// a caller-supplied compose file. Plain `create` implies it.
    CreateFromBlueprint(String),
}

impl PermissionOrWildcard {
    const CREATE_FROM_PREFIX: &'static str = "create:";

    /// The policy string, e.g. `view`, `*` or `create:static-html`.
    pub fn as_policy_string(&self) -> String {
        match self {
            Self::Permission(perm) => perm.as_str().to_string(),
            Self::Wildcard => "*".to_string(),
            Self::CreateFromBlueprint(bp) => format!("{}{bp}", Self::CREATE_FROM_PREFIX),
        }
    }
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

/// Configuration structure for saving (excludes dynamically managed apps).
///
/// Uses `BTreeMap` so keys are serialized in a stable, sorted order. This keeps
/// the on-disk `policy.yaml` byte-identical across saves when nothing changed,
/// avoiding spurious diffs from `HashMap`'s randomized iteration order.
#[derive(Debug, Clone, Serialize)]
pub struct AuthConfigForSave {
    pub scopes: BTreeMap<String, ScopeConfig>,
    pub roles: BTreeMap<String, RoleConfig>,
    pub assignments: BTreeMap<String, Vec<Assignment>>,
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
        let strings: Vec<String> = perms.iter().map(|p| p.as_policy_string()).collect();
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
                } else if let Some(bp) = s
                    .strip_prefix(PermissionOrWildcard::CREATE_FROM_PREFIX)
                    .filter(|bp| !bp.is_empty())
                {
                    PermissionOrWildcard::CreateFromBlueprint(bp.to_string())
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

#[cfg(test)]
mod create_from_blueprint_tests {
    use super::*;

    #[test]
    fn create_from_blueprint_parses_and_round_trips() {
        let role: RoleConfig =
            serde_norway::from_str("permissions: [view, 'create:static-html']\ndescription: d\n")
                .unwrap();
        // Before this variant existed, unknown strings became `*`.
        assert_eq!(
            role.permissions,
            vec![
                PermissionOrWildcard::Permission(Permission::View),
                PermissionOrWildcard::CreateFromBlueprint("static-html".into()),
            ]
        );
        let yaml = serde_norway::to_string(&role).unwrap();
        assert!(yaml.contains("create:static-html"), "{yaml}");
    }
}
