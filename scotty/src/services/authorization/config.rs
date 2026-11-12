use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

use super::types::{
    AuthConfig, AuthConfigForSave, Permission, PermissionOrWildcard, RoleConfig, ScopeConfig,
};

/// Configuration loading and management functionality
pub struct ConfigManager;

impl ConfigManager {
    /// Load configuration from YAML file.
    ///
    /// `policy.yaml` is mutated at runtime (admin scope/role/assignment
    /// changes) and is therefore gitignored. When it's missing we seed it from
    /// the tracked `policy.yaml.example` template if one exists; otherwise we
    /// fall back to the in-code defaults.
    pub async fn load_config(path: &str) -> Result<AuthConfig> {
        if !Path::new(path).exists() {
            let example_path = format!("{path}.example");
            if Path::new(&example_path).exists() {
                info!(
                    "Authorization config not found at {}, seeding from {}",
                    path, example_path
                );
                tokio::fs::copy(&example_path, path)
                    .await
                    .with_context(|| format!("Failed to seed {path} from {example_path}"))?;
            } else {
                warn!("Authorization config not found at {}, using defaults", path);
                return Ok(Self::default_config());
            }
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read authorization config")?;

        serde_norway::from_str(&content).context("Failed to parse authorization config")
    }

    /// Save configuration to file (excluding apps which are managed dynamically)
    pub async fn save_config(config: &AuthConfig, config_path: &str) -> Result<()> {
        // Create a config without apps for saving. Collecting into the
        // BTreeMap fields sorts keys deterministically, keeping policy.yaml
        // stable across saves regardless of HashMap iteration order.
        let save_config = AuthConfigForSave {
            scopes: config.scopes.clone().into_iter().collect(),
            roles: config.roles.clone().into_iter().collect(),
            assignments: config.assignments.clone().into_iter().collect(),
        };

        let yaml = serde_norway::to_string(&save_config)?;
        // Write a sibling of the real file (through symlinks) and rename it
        // over the policy, so a crash never leaves a half-written policy.yaml.
        let target = tokio::fs::canonicalize(config_path)
            .await
            .unwrap_or_else(|_| config_path.into());
        let tmp_path = target.with_extension("yaml.tmp");
        tokio::fs::write(&tmp_path, &yaml)
            .await
            .context("Failed to save authorization config")?;
        if let Err(e) = tokio::fs::rename(&tmp_path, &target).await {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            // A file bind-mounted on its own can't be replaced; write in place.
            warn!(
                "Can't replace {}: {e}; writing it in place",
                target.display()
            );
            tokio::fs::write(&target, yaml)
                .await
                .context("Failed to save authorization config")?;
        }
        Ok(())
    }

    /// Create default configuration when no config file exists
    fn default_config() -> AuthConfig {
        AuthConfig {
            scopes: HashMap::from([(
                "default".to_string(),
                ScopeConfig {
                    description: "Default scope".to_string(),
                    created_at: Utc::now(),
                },
            )]),
            roles: HashMap::from([
                (
                    "admin".to_string(),
                    RoleConfig {
                        permissions: vec![PermissionOrWildcard::Wildcard],
                        description: "Administrator".to_string(),
                    },
                ),
                (
                    "developer".to_string(),
                    RoleConfig {
                        permissions: vec![
                            PermissionOrWildcard::Permission(Permission::View),
                            PermissionOrWildcard::Permission(Permission::Manage),
                            PermissionOrWildcard::Permission(Permission::Shell),
                            PermissionOrWildcard::Permission(Permission::Logs),
                            PermissionOrWildcard::Permission(Permission::Create),
                        ],
                        description: "Developer access".to_string(),
                    },
                ),
                (
                    "operator".to_string(),
                    RoleConfig {
                        permissions: vec![
                            PermissionOrWildcard::Permission(Permission::View),
                            PermissionOrWildcard::Permission(Permission::Manage),
                            PermissionOrWildcard::Permission(Permission::Logs),
                        ],
                        description: "Operations access".to_string(),
                    },
                ),
                (
                    "viewer".to_string(),
                    RoleConfig {
                        permissions: vec![PermissionOrWildcard::Permission(Permission::View)],
                        description: "Read-only access".to_string(),
                    },
                ),
            ]),
            assignments: HashMap::new(),
            apps: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn example_policy_loads_with_blueprint_scoped_create() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../config/casbin/policy.yaml.example"
        );
        let config = ConfigManager::load_config(path).await.unwrap();
        let role = &config.roles["artifact_publisher"];
        assert!(role
            .permissions
            .contains(&PermissionOrWildcard::CreateFromBlueprint(
                "static-html".into()
            )));
        assert!(!role.permissions.contains(&PermissionOrWildcard::Wildcard));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn save_keeps_a_symlinked_policy_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.yaml");
        let link = dir.path().join("policy.yaml");
        std::fs::write(&real, "old").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let config = ConfigManager::default_config();
        ConfigManager::save_config(&config, link.to_str().unwrap())
            .await
            .unwrap();
        assert!(std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(std::fs::read_to_string(&real).unwrap().contains("default"));
    }
}
