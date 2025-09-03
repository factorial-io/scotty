use anyhow::{Context, Result};
use casbin::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::casbin::CasbinManager;
use super::config::ConfigManager;
use super::fallback::FallbackService;
use super::types::{
    Assignment, AuthConfig, Permission, PermissionOrWildcard, RoleConfig, ScopeConfig,
};

/// Casbin-based authorization service
pub struct AuthorizationService {
    enforcer: Arc<RwLock<CachedEnforcer>>,
    config: Arc<RwLock<AuthConfig>>,
    config_path: String,
}

impl AuthorizationService {
    /// Create a new authorization service with Casbin
    pub async fn new(config_dir: &str) -> Result<Self> {
        let model_path = format!("{}/model.conf", config_dir);
        let policy_path = format!("{}/policy.yaml", config_dir);

        // Load configuration from YAML
        let config = ConfigManager::load_config(&policy_path).await?;

        // Create Casbin enforcer using DefaultModel and MemoryAdapter
        let m = DefaultModel::from_file(&model_path)
            .await
            .context("Failed to load Casbin model")?;

        let a = MemoryAdapter::default();
        let mut enforcer = CachedEnforcer::new(m, a)
            .await
            .context("Failed to create Casbin enforcer")?;

        // Load policies into Casbin
        CasbinManager::sync_policies_to_casbin(&mut enforcer, &config).await?;

        info!(
            "Authorization service initialized with {} scopes, {} roles",
            config.scopes.len(),
            config.roles.len()
        );

        Ok(Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
            config: Arc::new(RwLock::new(config)),
            config_path: policy_path,
        })
    }

    /// Create a fallback authorization service with minimal configuration
    pub async fn create_fallback_service(legacy_access_token: Option<String>) -> Self {
        FallbackService::create_fallback_service(legacy_access_token).await
    }

    /// Create service from existing components (used by fallback service)
    pub fn new_from_components(
        enforcer: Arc<RwLock<CachedEnforcer>>,
        config: Arc<RwLock<AuthConfig>>,
        config_path: String,
    ) -> Self {
        Self {
            enforcer,
            config,
            config_path,
        }
    }

    /// Debug method to print the complete state of the authorization service
    #[allow(dead_code)] // Useful for debugging
    pub async fn debug_authorization_state(&self) {
        println!("=== AUTHORIZATION SERVICE COMPLETE STATE ===");
        println!("Config path: {}", self.config_path);

        let config = self.config.read().await;
        let enforcer = self.enforcer.read().await;

        // Print scopes
        println!("SCOPES:");
        for (scope_name, scope_config) in &config.scopes {
            println!("  - {}: {}", scope_name, scope_config.description);
        }

        // Print roles
        println!("ROLES:");
        for (role_name, role_config) in &config.roles {
            let perms: Vec<String> = role_config
                .permissions
                .iter()
                .map(|p| match p {
                    PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                    PermissionOrWildcard::Wildcard => "*".to_string(),
                })
                .collect();
            println!(
                "  - {}: [{}] - {}",
                role_name,
                perms.join(", "),
                role_config.description
            );
        }

        // Print user assignments
        println!("USER ASSIGNMENTS:");
        for (user_id, assignments) in &config.assignments {
            for assignment in assignments {
                println!(
                    "  - User '{}' has role '{}' in scopes: [{}]",
                    user_id,
                    assignment.role,
                    assignment.scopes.join(", ")
                );
            }
        }

        // Print app to scope mappings
        println!("APP SCOPE MAPPINGS:");
        for (app_name, scopes) in &config.apps {
            println!(
                "  - App '{}' is in scopes: [{}]",
                app_name,
                scopes.join(", ")
            );
        }

        // Print all Casbin policies
        println!("CASBIN POLICIES:");
        let policies = enforcer.get_policy();
        if policies.is_empty() {
            println!("  - NO POLICIES FOUND!");
        } else {
            for policy in &policies {
                println!("  - Policy: [{}]", policy.join(", "));
            }
        }

        // Print all Casbin user->role groupings (g)
        println!("CASBIN USER->ROLE GROUPINGS (g):");
        let user_role_groupings = enforcer.get_named_grouping_policy("g");
        if user_role_groupings.is_empty() {
            println!("  - NO USER->ROLE GROUPINGS FOUND!");
        } else {
            for grouping in &user_role_groupings {
                println!("  - User->Role: [{}]", grouping.join(", "));
            }
        }

        // Print all Casbin app->scope groupings (g2)
        println!("CASBIN APP->SCOPE GROUPINGS (g2):");
        let app_scope_groupings = enforcer.get_named_grouping_policy("g2");
        if app_scope_groupings.is_empty() {
            println!("  - NO APP->SCOPE GROUPINGS FOUND!");
        } else {
            for grouping in &app_scope_groupings {
                println!("  - App->Scope: [{}]", grouping.join(", "));
            }
        }

        println!("=== END AUTHORIZATION STATE ===");
    }

    /// Check if a user has permission to perform an action on an app
    pub async fn check_permission(&self, user: &str, app: &str, action: &Permission) -> bool {
        info!(
            "Checking permission: user='{}', app='{}', action='{}'",
            user,
            app,
            action.as_str()
        );
        let action_str = action.as_str();

        let enforcer = self.enforcer.read().await;

        let result = enforcer
            .enforce(vec![user, app, action_str])
            .unwrap_or(false);

        if result {
            info!("Permission granted: {} can {} on {}", user, action_str, app);
        } else {
            info!(
                "Permission denied: {} cannot {} on {}",
                user, action_str, app
            );
        }

        result
    }

    /// Check if a user has a global permission (not tied to a specific app)
    /// For global permissions like AdminRead/AdminWrite, this checks if the user has the permission
    /// across any of their scopes rather than requiring a specific app
    pub async fn check_global_permission(&self, user: &str, action: &Permission) -> bool {
        info!(
            "Checking global permission: user='{}', action='{}'",
            user,
            action.as_str()
        );

        let config = self.config.read().await;

        // Get user assignments
        let all_assignments = [
            config
                .assignments
                .get(user)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            config
                .assignments
                .get("*")
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        ]
        .concat();

        // Check if user has the permission in any of their roles
        for assignment in &all_assignments {
            if let Some(role_config) = config.roles.get(&assignment.role) {
                let has_permission = role_config.permissions.iter().any(|p| match p {
                    PermissionOrWildcard::Wildcard => true,
                    PermissionOrWildcard::Permission(perm) => perm == action,
                });

                if has_permission {
                    info!(
                        "Global permission granted: {} has {} via role {}",
                        user,
                        action.as_str(),
                        assignment.role
                    );
                    return true;
                }
            }
        }

        info!(
            "Global permission denied: {} lacks {}",
            user,
            action.as_str()
        );
        false
    }

    /// Format user identifier for authorization checks
    pub fn format_user_id(email: &str, token: Option<&str>) -> String {
        if let Some(token) = token {
            format!("bearer:{}", token)
        } else {
            email.to_string()
        }
    }

    /// Get the correct user ID for authorization checks from a CurrentUser
    /// Handles both bearer token users (with identifier: format) and OAuth users
    pub fn get_user_id_for_authorization(user: &crate::api::basic_auth::CurrentUser) -> String {
        if user.email.starts_with("identifier:") {
            // Bearer token users already have the correct identifier format
            user.email.clone()
        } else {
            // OAuth users use their email address directly
            user.email.clone()
        }
    }

    /// Check if authorization is enabled (has any assignments)
    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        !config.assignments.is_empty()
    }

    /// Look up user information by bearer token
    pub async fn get_user_by_token(&self, token: &str) -> Option<String> {
        let config = self.config.read().await;
        let token_user_id = Self::format_user_id("", Some(token));

        // Only authenticate tokens that are explicitly listed in assignments
        if config.assignments.contains_key(&token_user_id) {
            Some(token_user_id)
        } else {
            None
        }
    }

    /// Look up user information by identifier (new format: identifier:admin)
    pub async fn get_user_by_identifier(&self, identifier_user_id: &str) -> Option<String> {
        let config = self.config.read().await;

        // Check if user assignments exist for this identifier
        if config.assignments.contains_key(identifier_user_id) {
            Some(identifier_user_id.to_string())
        } else {
            None
        }
    }

    /// Save current configuration to file
    async fn save_config(&self) -> Result<()> {
        // Skip saving for fallback service (in-memory only)
        if self.config_path.starts_with("fallback/") {
            return Ok(());
        }

        let config = self.config.read().await;
        ConfigManager::save_config(&config, &self.config_path).await
    }

    /// Get all available scopes defined in the authorization configuration
    pub async fn get_scopes(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.scopes.keys().cloned().collect()
    }

    /// Validate that all specified scopes exist in the authorization system
    /// Returns Ok(()) if all scopes exist, or Err with missing scopes if not
    pub async fn validate_scopes(&self, scopes: &[String]) -> Result<(), Vec<String>> {
        let available_scopes = self.get_scopes().await;
        let missing_scopes: Vec<String> = scopes
            .iter()
            .filter(|scope| !available_scopes.contains(scope))
            .cloned()
            .collect();

        if missing_scopes.is_empty() {
            Ok(())
        } else {
            Err(missing_scopes)
        }
    }

    /// Get all scopes a user has access to with their permissions
    pub async fn get_user_scopes_with_permissions(
        &self,
        user: &str,
    ) -> Vec<crate::api::handlers::scopes::list::ScopeInfo> {
        let config = self.config.read().await;
        let mut user_scopes = Vec::new();

        // Collect assignments from both specific user and wildcard "*"
        let mut all_assignments = Vec::new();

        // Add wildcard assignments (everyone gets these)
        if let Some(wildcard_assignments) = config.assignments.get("*") {
            all_assignments.extend(wildcard_assignments.iter());
        }

        // Add user-specific assignments
        if let Some(user_assignments) = config.assignments.get(user) {
            all_assignments.extend(user_assignments.iter());
        }

        // Process all assignments
        for assignment in all_assignments {
            // Get role permissions
            let permissions = if let Some(role_config) = config.roles.get(&assignment.role) {
                role_config
                    .permissions
                    .iter()
                    .map(|p| match p {
                        PermissionOrWildcard::Wildcard => "*".to_string(),
                        PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                    })
                    .collect()
            } else {
                vec![]
            };

            // Add each group the user has access to
            for scope in &assignment.scopes {
                let scope_info = crate::api::handlers::scopes::list::ScopeInfo {
                    name: scope.clone(),
                    description: config
                        .scopes
                        .get(scope)
                        .map(|s| s.description.clone())
                        .unwrap_or_else(|| format!("Scope: {}", scope)),
                    permissions: permissions.clone(),
                };

                // Only add if not already in the list (user might have multiple roles for same scope)
                if !user_scopes
                    .iter()
                    .any(|s: &crate::api::handlers::scopes::list::ScopeInfo| {
                        s.name == scope_info.name
                    })
                {
                    user_scopes.push(scope_info);
                } else {
                    // If scope already exists, merge permissions
                    if let Some(existing) = user_scopes.iter_mut().find(
                        |s: &&mut crate::api::handlers::scopes::list::ScopeInfo| s.name == *scope,
                    ) {
                        for perm in &permissions {
                            if !existing.permissions.contains(perm) {
                                existing.permissions.push(perm.clone());
                            }
                        }
                    }
                }
            }
        }

        user_scopes
    }

    /// Assign an app to scopes
    /// Note: Caller should validate scopes exist using validate_scopes() before calling this
    pub async fn set_app_scopes(&self, app: &str, scopes: Vec<String>) -> Result<()> {
        let mut config = self.config.write().await;
        let mut enforcer = self.enforcer.write().await;

        // Remove existing app-scope associations from Casbin (g2 policies)
        let existing_policies = enforcer.get_named_grouping_policy("g2");
        let app_policies: Vec<_> = existing_policies
            .iter()
            .filter(|p| p.len() >= 2 && p[0] == app)
            .cloned()
            .collect();
        for policy in app_policies {
            enforcer.remove_named_grouping_policy("g2", policy).await?;
        }

        // Add new app-scope associations to Casbin (g2 policies)
        for scope in &scopes {
            enforcer
                .add_named_grouping_policy("g2", vec![app.to_string(), scope.clone()])
                .await?;
        }

        // Update config
        config.apps.insert(app.to_string(), scopes);

        // Save config
        drop(config);
        drop(enforcer);
        self.save_config().await?;

        Ok(())
    }

    /// Create a new scope
    pub async fn create_scope(&self, name: &str, description: &str) -> Result<()> {
        let mut config = self.config.write().await;

        if config.scopes.contains_key(name) {
            anyhow::bail!("Scope '{}' already exists", name);
        }

        config.scopes.insert(
            name.to_string(),
            ScopeConfig {
                description: description.to_string(),
                created_at: chrono::Utc::now(),
            },
        );

        drop(config);
        self.save_config().await?;

        info!("Created scope '{}'", name);
        Ok(())
    }

    /// Get all scopes
    pub async fn list_scopes(&self) -> Vec<(String, ScopeConfig)> {
        let config = self.config.read().await;
        config
            .scopes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Create a new role
    pub async fn create_role(
        &self,
        name: &str,
        permissions: Vec<PermissionOrWildcard>,
        description: &str,
    ) -> Result<()> {
        let mut config = self.config.write().await;

        if config.roles.contains_key(name) {
            anyhow::bail!("Role '{}' already exists", name);
        }

        config.roles.insert(
            name.to_string(),
            RoleConfig {
                permissions,
                description: description.to_string(),
            },
        );

        drop(config);
        self.save_config().await?;

        info!("Created role '{}'", name);
        Ok(())
    }

    /// Assign role to user for specific scopes
    pub async fn assign_user_role(
        &self,
        user: &str,
        role: &str,
        scopes: Vec<String>,
    ) -> Result<()> {
        let mut config = self.config.write().await;
        let mut enforcer = self.enforcer.write().await;

        // Check if role exists
        if !config.roles.contains_key(role) {
            anyhow::bail!("Role '{}' does not exist", role);
        }

        // Note: We now use direct user-scope-permission policies instead of user->role mappings

        // Add user permissions for each scope to Casbin (direct user-scope-permission policies)
        if let Some(role_config) = config.roles.get(role) {
            for scope in &scopes {
                for permission in &role_config.permissions {
                    match permission {
                        PermissionOrWildcard::Wildcard => {
                            // Add all permissions for this user-scope combination
                            for perm in Permission::all() {
                                let action = perm.as_str();
                                enforcer
                                    .add_policy(vec![
                                        user.to_string(),
                                        scope.clone(),
                                        action.to_string(),
                                    ])
                                    .await?;
                            }
                        }
                        PermissionOrWildcard::Permission(perm) => {
                            let action = perm.as_str();
                            enforcer
                                .add_policy(vec![
                                    user.to_string(),
                                    scope.clone(),
                                    action.to_string(),
                                ])
                                .await?;
                        }
                    }
                }
            }
        }

        // Update config
        let assignments = config
            .assignments
            .entry(user.to_string())
            .or_insert_with(Vec::new);

        // Check if assignment already exists
        if !assignments
            .iter()
            .any(|a| a.role == role && a.scopes == scopes)
        {
            assignments.push(Assignment {
                role: role.to_string(),
                scopes,
            });
        }

        drop(config);
        drop(enforcer);
        self.save_config().await?;

        info!("Assigned role '{}' to user '{}'", role, user);
        Ok(())
    }

    /// Get user's effective permissions for debugging
    pub async fn get_user_permissions(&self, user: &str) -> HashMap<String, Vec<String>> {
        let config = self.config.read().await;
        let mut all_permissions: HashMap<String, Vec<String>> = HashMap::new();

        // Collect assignments from both specific user and wildcard "*"
        let mut all_assignments = Vec::new();

        // Add wildcard assignments (everyone gets these)
        if let Some(wildcard_assignments) = config.assignments.get("*") {
            all_assignments.extend(wildcard_assignments.iter());
        }

        // Add user-specific assignments
        if let Some(user_assignments) = config.assignments.get(user) {
            all_assignments.extend(user_assignments.iter());
        }

        // Process all assignments
        for assignment in all_assignments {
            // Get role permissions
            if let Some(role_config) = config.roles.get(&assignment.role) {
                let permissions: Vec<String> = role_config
                    .permissions
                    .iter()
                    .map(|p| match p {
                        PermissionOrWildcard::Wildcard => "*".to_string(),
                        PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                    })
                    .collect();

                // Add permissions for each scope
                for scope in &assignment.scopes {
                    let scope_perms = all_permissions.entry(scope.clone()).or_default();
                    for perm in &permissions {
                        if !scope_perms.contains(perm) {
                            scope_perms.push(perm.clone());
                        }
                    }
                }
            }
        }

        all_permissions
    }

    /// Get all roles
    pub async fn list_roles(&self) -> Vec<(String, RoleConfig)> {
        let config = self.config.read().await;
        config
            .roles
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get all assignments
    pub async fn list_assignments(&self) -> HashMap<String, Vec<Assignment>> {
        let config = self.config.read().await;
        config.assignments.clone()
    }

    /// Get enforcer for testing (internal use only)
    #[cfg(test)]
    pub async fn get_enforcer_for_testing(&self) -> Arc<RwLock<CachedEnforcer>> {
        self.enforcer.clone()
    }
}

impl std::fmt::Debug for AuthorizationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationService")
            .field("config_path", &self.config_path)
            .finish_non_exhaustive()
    }
}
