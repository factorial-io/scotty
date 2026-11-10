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

    /// Parses a policy string. Unknown or malformed strings are `None`, never
    /// `*`: a typo must not grant everything.
    pub fn parse(s: &str) -> Option<Self> {
        if s == "*" {
            return Some(Self::Wildcard);
        }
        if let Some(bp) = s.strip_prefix(Self::CREATE_FROM_PREFIX) {
            let valid = !bp.is_empty()
                && bp
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            return valid.then(|| Self::CreateFromBlueprint(bp.to_string()));
        }
        Permission::from_str(s).map(Self::Permission)
    }

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
    use super::PermissionOrWildcard;
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
        strings
            .into_iter()
            .map(|s| {
                PermissionOrWildcard::parse(&s)
                    .ok_or_else(|| serde::de::Error::custom(format!("unknown permission '{s}'")))
            })
            .collect()
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

    #[test]
    fn unknown_or_malformed_permissions_are_errors_not_wildcards() {
        for bad in [
            "create:",
            "create :static-html",
            "create:a b",
            "creat",
            "viewer",
        ] {
            let yaml = format!("permissions: ['{bad}']\ndescription: d\n");
            let err = serde_norway::from_str::<RoleConfig>(&yaml).unwrap_err();
            assert!(
                err.to_string().contains("unknown permission"),
                "{bad}: {err}"
            );
            assert_eq!(PermissionOrWildcard::parse(bad), None, "{bad}");
        }
    }
}
