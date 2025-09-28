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

        // Ensure all scopes from config are available (even if no apps assigned yet)
        info!("Loading scopes from policy config:");
        for (scope_name, scope_config) in &config.scopes {
            info!("  - Scope: {} ({})", scope_name, scope_config.description);
        }

        // Add app -> scope mappings (g2 groupings)
        info!("Adding app -> scope mappings:");
        for (app, scopes) in &config.apps {
            for scope in scopes {
                info!("Adding g2: {} -> {}", app, scope);
                enforcer
                    .add_named_grouping_policy("g2", vec![app.to_string(), scope.to_string()])
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

                // Add user permissions for each scope (direct user-scope-permission policies)
                if let Some(role_config) = config.roles.get(&assignment.role) {
                    // Expand wildcard scopes to actual scopes
                    let expanded_scopes = Self::expand_wildcard_scopes(&assignment.scopes, &config.scopes);

                    for scope in &expanded_scopes {
                        for permission in &role_config.permissions {
                            Self::add_permission_policies(enforcer, user, scope, permission)
                                .await?;
                        }
                    }
                }
            }
        }

        info!("Casbin policy synchronization completed");

        Ok(())
    }

    /// Helper method to expand wildcard scopes to actual scope names
    fn expand_wildcard_scopes(scopes: &[String], available_scopes: &std::collections::HashMap<String, super::types::ScopeConfig>) -> Vec<String> {
        if scopes.contains(&"*".to_string()) {
            available_scopes.keys().cloned().collect()
        } else {
            scopes.to_vec()
        }
    }

    /// Add permission policies for a user-scope combination
    async fn add_permission_policies(
        enforcer: &mut CachedEnforcer,
        user: &str,
        scope: &str,
        permission: &PermissionOrWildcard,
    ) -> Result<()> {
        match permission {
            PermissionOrWildcard::Wildcard => {
                // Add all permissions for this user-scope combination
                for perm in Permission::all() {
                    let action = perm.as_str();
                    info!(
                        "Adding p: {} {} {} (user-scope policy)",
                        user, scope, action
                    );
                    enforcer
                        .add_policy(vec![
                            user.to_string(),
                            scope.to_string(),
                            action.to_string(),
                        ])
                        .await?;
                }
            }
            PermissionOrWildcard::Permission(perm) => {
                let action = perm.as_str();
                info!(
                    "Adding p: {} {} {} (user-scope policy)",
                    user, scope, action
                );
                enforcer
                    .add_policy(vec![
                        user.to_string(),
                        scope.to_string(),
                        action.to_string(),
                    ])
                    .await?;
            }
        }
        Ok(())
    }
}
