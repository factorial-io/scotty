use anyhow::Result;
use casbin::prelude::*;
use tracing::info;

use super::types::{AuthConfig, Permission, PermissionOrWildcard};

/// Casbin-specific operations and policy management
pub struct CasbinManager;

impl CasbinManager {
    /// Synchronize YAML config to Casbin policies
    pub async fn sync_policies_to_casbin(
        enforcer: &mut CachedEnforcer,
        config: &AuthConfig,
    ) -> Result<()> {
        info!("Starting Casbin policy synchronization");

        // Clear existing policies
        let _ = enforcer.clear_policy().await;

        // Ensure all groups from config are available (even if no apps assigned yet)
        info!("Loading groups from policy config:");
        for (group_name, group_config) in &config.groups {
            info!("  - Group: {} ({})", group_name, group_config.description);
        }

        // Add app -> group mappings (g2 groupings)
        info!("Adding app -> group mappings:");
        for (app, groups) in &config.apps {
            for group in groups {
                info!("Adding g2: {} -> {}", app, group);
                enforcer
                    .add_named_grouping_policy("g2", vec![app.to_string(), group.to_string()])
                    .await?;
            }
        }

        // Add user -> role mappings and role -> permissions
        info!("Adding user -> role mappings and permissions:");
        for (user, assignments) in &config.assignments {
            for assignment in assignments {
                // Add user to role (g groupings)
                info!("Adding g: {} -> {}", user, assignment.role);
                enforcer
                    .add_named_grouping_policy("g", vec![user.to_string(), assignment.role.clone()])
                    .await?;

                // Add user permissions for each group (direct user-group-permission policies)
                if let Some(role_config) = config.roles.get(&assignment.role) {
                    for group in &assignment.groups {
                        for permission in &role_config.permissions {
                            Self::add_permission_policies(enforcer, user, group, permission)
                                .await?;
                        }
                    }
                }
            }
        }

        info!("Casbin policy synchronization completed");

        Ok(())
    }

    /// Add permission policies for a user-group combination
    async fn add_permission_policies(
        enforcer: &mut CachedEnforcer,
        user: &str,
        group: &str,
        permission: &PermissionOrWildcard,
    ) -> Result<()> {
        match permission {
            PermissionOrWildcard::Wildcard => {
                // Add all permissions for this user-group combination
                for perm in Permission::all() {
                    let action = perm.as_str();
                    info!(
                        "Adding p: {} {} {} (user-group policy)",
                        user, group, action
                    );
                    enforcer
                        .add_policy(vec![
                            user.to_string(),
                            group.to_string(),
                            action.to_string(),
                        ])
                        .await?;
                }
            }
            PermissionOrWildcard::Permission(perm) => {
                let action = perm.as_str();
                info!(
                    "Adding p: {} {} {} (user-group policy)",
                    user, group, action
                );
                enforcer
                    .add_policy(vec![
                        user.to_string(),
                        group.to_string(),
                        action.to_string(),
                    ])
                    .await?;
            }
        }
        Ok(())
    }
}
