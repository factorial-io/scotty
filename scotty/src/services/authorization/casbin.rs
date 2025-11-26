use anyhow::Result;
use casbin::prelude::*;
use casbin::rhai::Dynamic;
use tracing::info;

use super::types::{AuthConfig, Permission, PermissionOrWildcard};

/// Casbin-specific operations and policy management
pub struct CasbinManager;

/// Custom user matching function for Casbin matchers.
///
/// Matches request user (r_user) against policy user (p_user) with support for:
/// 1. Exact match (case-insensitive for emails)
/// 2. Domain pattern match (e.g., `@factorial.io` matches `user@factorial.io`)
/// 3. Wildcard match (`*` matches any user)
///
/// This function is registered with Casbin via `add_function("user_match", ...)`.
pub fn user_match(r_user: Dynamic, p_user: Dynamic) -> Dynamic {
    let r_user_str = r_user.into_string().unwrap_or_default();
    let p_user_str = p_user.into_string().unwrap_or_default();

    let result = user_match_impl(&r_user_str, &p_user_str);
    result.into()
}

/// Implementation of user matching logic (separated for easier testing)
pub fn user_match_impl(r_user: &str, p_user: &str) -> bool {
    // Normalize emails to lowercase for case-insensitive matching (RFC 5321)
    let r_user_normalized = if r_user.contains('@') {
        r_user.to_lowercase()
    } else {
        r_user.to_string()
    };

    let p_user_normalized = if p_user.contains('@') {
        p_user.to_lowercase()
    } else {
        p_user.to_string()
    };

    // 1. Wildcard match - policy user "*" matches any request user
    if p_user_normalized == "*" {
        return true;
    }

    // 2. Exact match (case-insensitive for emails)
    if r_user_normalized == p_user_normalized {
        return true;
    }

    // 3. Domain pattern match - policy user "@domain.com" matches "user@domain.com"
    if p_user_normalized.starts_with('@') {
        // Extract domain from request user email
        if let Some(at_pos) = r_user_normalized.find('@') {
            let r_domain = &r_user_normalized[at_pos..]; // includes the @
            if r_domain == p_user_normalized {
                return true;
            }
        }
    }

    false
}

/// Register the custom user_match function with a Casbin enforcer
pub fn register_user_match_function(enforcer: &mut CachedEnforcer) {
    use casbin::function_map::OperatorFunction;
    enforcer.add_function("user_match", OperatorFunction::Arg2(user_match));
}

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
                    let expanded_scopes =
                        Self::expand_wildcard_scopes(&assignment.scopes, &config.scopes);

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
    fn expand_wildcard_scopes(
        scopes: &[String],
        available_scopes: &std::collections::HashMap<String, super::types::ScopeConfig>,
    ) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_match_exact() {
        // Exact match (same case)
        assert!(user_match_impl("user@example.com", "user@example.com"));

        // Exact match (different case - should match due to normalization)
        assert!(user_match_impl("User@Example.com", "user@example.com"));
        assert!(user_match_impl("user@example.com", "User@Example.com"));

        // Non-email identifiers (case-sensitive)
        assert!(user_match_impl("identifier:admin", "identifier:admin"));
        assert!(!user_match_impl("identifier:Admin", "identifier:admin"));
    }

    #[test]
    fn test_user_match_domain_pattern() {
        // Domain pattern matches
        assert!(user_match_impl("user@factorial.io", "@factorial.io"));
        assert!(user_match_impl("another@factorial.io", "@factorial.io"));
        assert!(user_match_impl("User@Factorial.IO", "@factorial.io"));

        // Domain pattern doesn't match different domains
        assert!(!user_match_impl("user@other.com", "@factorial.io"));
        assert!(!user_match_impl(
            "user@factorial.io.evil.com",
            "@factorial.io"
        ));

        // Non-email shouldn't match domain patterns
        assert!(!user_match_impl("identifier:admin", "@factorial.io"));
    }

    #[test]
    fn test_user_match_wildcard() {
        // Wildcard matches everything
        assert!(user_match_impl("user@example.com", "*"));
        assert!(user_match_impl("identifier:admin", "*"));
        assert!(user_match_impl("anything", "*"));
        assert!(user_match_impl("", "*"));
    }

    #[test]
    fn test_user_match_no_match() {
        // Different users, no pattern match
        assert!(!user_match_impl("user@example.com", "other@example.com"));
        assert!(!user_match_impl("user@example.com", "user@other.com"));
        assert!(!user_match_impl("identifier:admin", "identifier:user"));
    }

    #[test]
    fn test_user_match_security() {
        // Ensure domain pattern doesn't match subdomain attacks
        assert!(!user_match_impl("user@evil.factorial.io", "@factorial.io"));

        // Ensure partial matches don't work
        assert!(!user_match_impl("userfactorial.io", "@factorial.io"));
    }
}
