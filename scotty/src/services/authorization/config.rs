use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use tracing::warn;

use super::types::{
    AuthConfig, AuthConfigForSave, GroupConfig, Permission, PermissionOrWildcard, RoleConfig,
};

/// Configuration loading and management functionality
pub struct ConfigManager;

impl ConfigManager {
    /// Load configuration from YAML file
    pub async fn load_config(path: &str) -> Result<AuthConfig> {
        if !Path::new(path).exists() {
            warn!("Authorization config not found at {}, using defaults", path);
            return Ok(Self::default_config());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read authorization config")?;

        serde_yml::from_str(&content).context("Failed to parse authorization config")
    }

    /// Save configuration to file (excluding apps which are managed dynamically)
    pub async fn save_config(config: &AuthConfig, config_path: &str) -> Result<()> {
        // Create a config without apps for saving
        let save_config = AuthConfigForSave {
            groups: config.groups.clone(),
            roles: config.roles.clone(),
            assignments: config.assignments.clone(),
        };

        let yaml = serde_yml::to_string(&save_config)?;
        tokio::fs::write(config_path, yaml)
            .await
            .context("Failed to save authorization config")?;
        Ok(())
    }

    /// Create default configuration when no config file exists
    fn default_config() -> AuthConfig {
        AuthConfig {
            groups: HashMap::from([(
                "default".to_string(),
                GroupConfig {
                    description: "Default group".to_string(),
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
