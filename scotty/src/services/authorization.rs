use anyhow::{Context, Result};
use casbin::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Available permissions/actions for authorization
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    View,
    Manage,
    Shell,
    Logs,
    Create,
    Destroy,
}

impl Permission {
    /// Get all available permissions
    pub fn all() -> Vec<Permission> {
        vec![
            Permission::View,
            Permission::Manage,
            Permission::Shell,
            Permission::Logs,
            Permission::Create,
            Permission::Destroy,
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
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Permission> {
        match s.to_lowercase().as_str() {
            "view" => Some(Permission::View),
            "manage" => Some(Permission::Manage),
            "shell" => Some(Permission::Shell),
            "logs" => Some(Permission::Logs),
            "create" => Some(Permission::Create),
            "destroy" => Some(Permission::Destroy),
            _ => None,
        }
    }
}

/// Authorization configuration loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub groups: HashMap<String, GroupConfig>,
    pub roles: HashMap<String, RoleConfig>,
    pub assignments: HashMap<String, Vec<Assignment>>,
    pub apps: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    #[serde(with = "permission_serde")]
    pub permissions: Vec<PermissionOrWildcard>,
    pub description: String,
}

/// Represents either a specific permission or wildcard (*)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOrWildcard {
    Permission(Permission),
    Wildcard,
}

mod permission_serde {
    use super::{Permission, PermissionOrWildcard};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(perms: &Vec<PermissionOrWildcard>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<String> = perms
            .iter()
            .map(|p| match p {
                PermissionOrWildcard::Permission(perm) => perm.as_str().to_string(),
                PermissionOrWildcard::Wildcard => "*".to_string(),
            })
            .collect();
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub role: String,
    pub groups: Vec<String>,
}

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
        let config = Self::load_config(&policy_path).await?;

        // Create Casbin enforcer using DefaultModel and MemoryAdapter
        let m = DefaultModel::from_file(&model_path)
            .await
            .context("Failed to load Casbin model")?;

        let a = MemoryAdapter::default();
        let mut enforcer = CachedEnforcer::new(m, a)
            .await
            .context("Failed to create Casbin enforcer")?;

        // Load policies into Casbin
        Self::sync_policies_to_casbin(&mut enforcer, &config).await?;

        info!(
            "Authorization service initialized with {} groups, {} roles",
            config.groups.len(),
            config.roles.len()
        );

        Ok(Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
            config: Arc::new(RwLock::new(config)),
            config_path: policy_path,
        })
    }

    /// Create a new authorization service with fallback configuration
    /// This is used when the main configuration can't be loaded.
    /// It creates a default setup where everyone has access to the "default" group
    /// and uses the legacy access token from settings if provided.
    pub async fn new_with_fallback(config_dir: &str, legacy_access_token: Option<String>) -> Self {
        match Self::new(config_dir).await {
            Ok(service) => {
                info!("Authorization service loaded successfully from config");
                service
            }
            Err(e) => {
                warn!(
                    "Failed to load authorization config: {}. Using fallback configuration.",
                    e
                );
                Self::create_fallback_service(legacy_access_token).await
            }
        }
    }

    /// Create a fallback authorization service with minimal configuration
    pub async fn create_fallback_service(legacy_access_token: Option<String>) -> Self {
        // Create a minimal Casbin model in memory
        let model_text = r#"
[request_definition]
r = sub, app, act

[policy_definition]
p = sub, group, act

[role_definition]
g = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && g(r.app, p.group) && r.act == p.act
"#;

        let m = DefaultModel::from_str(model_text)
            .await
            .expect("Failed to create fallback Casbin model");

        let a = MemoryAdapter::default();
        let mut enforcer = CachedEnforcer::new(m, a)
            .await
            .expect("Failed to create fallback Casbin enforcer");

        // Create default configuration with everyone having access to "default" group
        let mut config = AuthConfig {
            groups: HashMap::from([(
                "default".to_string(),
                GroupConfig {
                    description: "Default group for all users".to_string(),
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
                    "user".to_string(),
                    RoleConfig {
                        permissions: vec![
                            PermissionOrWildcard::Permission(Permission::View),
                            PermissionOrWildcard::Permission(Permission::Manage),
                            PermissionOrWildcard::Permission(Permission::Logs),
                        ],
                        description: "Regular user".to_string(),
                    },
                ),
            ]),
            assignments: HashMap::new(),
            apps: HashMap::new(),
        };

        // Add legacy access token if provided
        if let Some(token) = legacy_access_token {
            let user_id = Self::format_user_id("", Some(&token));
            config.assignments.insert(
                user_id,
                vec![Assignment {
                    role: "admin".to_string(),
                    groups: vec!["default".to_string()],
                }],
            );
        }

        // Assign all apps to default group and sync policies
        Self::sync_policies_to_casbin(&mut enforcer, &config)
            .await
            .expect("Failed to sync fallback policies to Casbin");

        info!("Fallback authorization service created with default configuration");

        Self {
            enforcer: Arc::new(RwLock::new(enforcer)),
            config: Arc::new(RwLock::new(config)),
            config_path: "fallback/policy.yaml".to_string(), // Placeholder path for fallback
        }
    }

    /// Load configuration from YAML file
    async fn load_config(path: &str) -> Result<AuthConfig> {
        if !Path::new(path).exists() {
            warn!("Authorization config not found at {}, using defaults", path);
            return Ok(AuthConfig {
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
            });
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read authorization config")?;

        serde_yml::from_str(&content).context("Failed to parse authorization config")
    }

    /// Synchronize YAML config to Casbin policies
    async fn sync_policies_to_casbin(
        enforcer: &mut CachedEnforcer,
        config: &AuthConfig,
    ) -> Result<()> {
        // Clear existing policies
        let _ = enforcer.clear_policy().await;

        // Add app -> group mappings (g2 groupings)
        for (app, groups) in &config.apps {
            for group in groups {
                debug!("Adding g2: {} -> {}", app, group);
                enforcer
                    .add_grouping_policy(vec![app.to_string(), group.to_string()])
                    .await?;
            }
        }

        // Add user -> role mappings and role -> permissions
        for (user, assignments) in &config.assignments {
            for assignment in assignments {
                // Add user to role (g groupings)
                debug!("Adding g: {} -> {}", user, assignment.role);
                enforcer
                    .add_grouping_policy(vec![user.to_string(), assignment.role.clone()])
                    .await?;

                // Add role permissions for each group
                if let Some(role_config) = config.roles.get(&assignment.role) {
                    for group in &assignment.groups {
                        for permission in &role_config.permissions {
                            match permission {
                                PermissionOrWildcard::Wildcard => {
                                    // Add all permissions
                                    for perm in Permission::all() {
                                        let action = perm.as_str();
                                        debug!(
                                            "Adding p: {} {} {}",
                                            assignment.role, group, action
                                        );
                                        enforcer
                                            .add_policy(vec![
                                                assignment.role.clone(),
                                                group.to_string(),
                                                action.to_string(),
                                            ])
                                            .await?;
                                    }
                                }
                                PermissionOrWildcard::Permission(perm) => {
                                    let action = perm.as_str();
                                    debug!("Adding p: {} {} {}", assignment.role, group, action);
                                    enforcer
                                        .add_policy(vec![
                                            assignment.role.clone(),
                                            group.to_string(),
                                            action.to_string(),
                                        ])
                                        .await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Save current configuration to file
    async fn save_config(&self) -> Result<()> {
        let config = self.config.read().await;
        let yaml = serde_yml::to_string(&*config)?;
        tokio::fs::write(&self.config_path, yaml)
            .await
            .context("Failed to save authorization config")?;
        Ok(())
    }

    /// Check if a user has permission to perform an action on an app
    pub async fn check_permission(&self, user: &str, app: &str, action: &Permission) -> bool {
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

    /// Get all groups an app belongs to
    pub async fn get_app_groups(&self, app: &str) -> Vec<String> {
        let config = self.config.read().await;
        config
            .apps
            .get(app)
            .cloned()
            .unwrap_or_else(|| vec!["default".to_string()])
    }

    /// Get all available groups defined in the authorization configuration
    pub async fn get_groups(&self) -> Vec<String> {
        let config = self.config.read().await;
        config.groups.keys().cloned().collect()
    }

    /// Validate that all specified groups exist in the authorization system
    /// Returns Ok(()) if all groups exist, or Err with missing groups if not
    pub async fn validate_groups(&self, groups: &[String]) -> Result<(), Vec<String>> {
        let available_groups = self.get_groups().await;
        let missing_groups: Vec<String> = groups
            .iter()
            .filter(|group| !available_groups.contains(group))
            .cloned()
            .collect();

        if missing_groups.is_empty() {
            Ok(())
        } else {
            Err(missing_groups)
        }
    }

    /// Get all groups a user has access to with their permissions
    pub async fn get_user_groups_with_permissions(
        &self,
        user: &str,
    ) -> Vec<crate::api::handlers::groups::list::GroupInfo> {
        let config = self.config.read().await;
        let mut user_groups = Vec::new();

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
            for group in &assignment.groups {
                let group_info = crate::api::handlers::groups::list::GroupInfo {
                    name: group.clone(),
                    description: config
                        .groups
                        .get(group)
                        .map(|g| g.description.clone())
                        .unwrap_or_else(|| format!("Group: {}", group)),
                    permissions: permissions.clone(),
                };

                // Only add if not already in the list (user might have multiple roles for same group)
                if !user_groups.iter().any(
                    |g: &crate::api::handlers::groups::list::GroupInfo| {
                        g.name == group_info.name
                    },
                ) {
                    user_groups.push(group_info);
                } else {
                    // If group already exists, merge permissions
                    if let Some(existing) = user_groups.iter_mut().find(
                        |g: &&mut crate::api::handlers::groups::list::GroupInfo| {
                            g.name == *group
                        },
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

        user_groups
    }

    /// Assign an app to groups
    /// Note: Caller should validate groups exist using validate_groups() before calling this
    pub async fn set_app_groups(&self, app: &str, groups: Vec<String>) -> Result<()> {
        let mut config = self.config.write().await;
        let mut enforcer = self.enforcer.write().await;

        // Remove existing app-group associations from Casbin (g policies)
        let existing_policies = enforcer.get_grouping_policy();
        let app_policies: Vec<_> = existing_policies
            .iter()
            .filter(|p| p.len() >= 2 && p[0] == app)
            .cloned()
            .collect();
        for policy in app_policies {
            enforcer.remove_grouping_policy(policy).await?;
        }

        // Add new app-group associations to Casbin (g policies)
        for group in &groups {
            enforcer
                .add_grouping_policy(vec![app.to_string(), group.clone()])
                .await?;
        }

        // Update config
        config.apps.insert(app.to_string(), groups);

        // Save config
        drop(config);
        drop(enforcer);
        self.save_config().await?;

        Ok(())
    }

    /// Create a new group
    pub async fn create_group(&self, name: &str, description: &str) -> Result<()> {
        let mut config = self.config.write().await;

        if config.groups.contains_key(name) {
            anyhow::bail!("Group '{}' already exists", name);
        }

        config.groups.insert(
            name.to_string(),
            GroupConfig {
                description: description.to_string(),
                created_at: Utc::now(),
            },
        );

        drop(config);
        self.save_config().await?;

        info!("Created group '{}'", name);
        Ok(())
    }

    /// Get all groups
    pub async fn list_groups(&self) -> Vec<(String, GroupConfig)> {
        let config = self.config.read().await;
        config
            .groups
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

    /// Assign role to user for specific groups
    pub async fn assign_user_role(
        &self,
        user: &str,
        role: &str,
        groups: Vec<String>,
    ) -> Result<()> {
        let mut config = self.config.write().await;
        let mut enforcer = self.enforcer.write().await;

        // Check if role exists
        if !config.roles.contains_key(role) {
            anyhow::bail!("Role '{}' does not exist", role);
        }

        // Note: We now use direct user-group-permission policies instead of user->role mappings

        // Add user permissions for each group to Casbin (direct user-group-permission policies)
        if let Some(role_config) = config.roles.get(role) {
            for group in &groups {
                for permission in &role_config.permissions {
                    match permission {
                        PermissionOrWildcard::Wildcard => {
                            // Add all permissions for this user-group combination
                            for perm in Permission::all() {
                                let action = perm.as_str();
                                enforcer
                                    .add_policy(vec![
                                        user.to_string(),
                                        group.clone(),
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
                                    group.clone(),
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
            .any(|a| a.role == role && a.groups == groups)
        {
            assignments.push(Assignment {
                role: role.to_string(),
                groups,
            });
        }

        drop(config);
        drop(enforcer);
        self.save_config().await?;

        info!("Assigned role '{}' to user '{}'", role, user);
        Ok(())
    }

    /// Format user identifier for authorization checks
    pub fn format_user_id(email: &str, token: Option<&str>) -> String {
        if let Some(token) = token {
            format!("bearer:{}", token)
        } else {
            email.to_string()
        }
    }

    /// Check if authorization is enabled (has any assignments)
    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        !config.assignments.is_empty()
    }

    /// Get user's effective permissions for debugging
    /// TODO: Implement this method when needed for debugging API
    #[allow(dead_code)]
    pub async fn get_user_permissions(&self, _user: &str) -> HashMap<String, Vec<String>> {
        // Simplified for now - will implement when needed
        HashMap::new()
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

    /// Look up user information by bearer token
    pub async fn get_user_by_token(&self, token: &str) -> Option<String> {
        let config = self.config.read().await;
        let token_user_id = Self::format_user_id("", Some(token));

        // Check if this token exists in assignments
        if config.assignments.contains_key(&token_user_id) {
            Some(token_user_id)
        } else {
            None
        }
    }
}

impl std::fmt::Debug for AuthorizationService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationService")
            .field("config_path", &self.config_path)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_service() -> (AuthorizationService, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_dir = temp_dir.path().to_str().unwrap();

        // Create model.conf
        let model_content = r#"[request_definition]
r = sub, app, act

[policy_definition]
p = sub, group, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub) && g2(r.app, p.group) && r.act == p.act
"#;
        tokio::fs::write(format!("{}/model.conf", config_dir), model_content)
            .await
            .unwrap();

        let service = AuthorizationService::new(config_dir).await.unwrap();
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_basic_authorization_flow() {
        let (service, _temp_dir) = create_test_service().await;

        // Create a group
        service
            .create_group("test-group", "Test group for authorization")
            .await
            .unwrap();

        // Set app to group
        service
            .set_app_groups("test-app", vec!["test-group".to_string()])
            .await
            .unwrap();

        // Assign developer role to user for test-group
        service
            .assign_user_role("test-user", "developer", vec!["test-group".to_string()])
            .await
            .unwrap();

        // Test permissions
        assert!(
            service
                .check_permission("test-user", "test-app", &Permission::View)
                .await
        );
        assert!(
            service
                .check_permission("test-user", "test-app", &Permission::Shell)
                .await
        );
        assert!(
            !service
                .check_permission("test-user", "test-app", &Permission::Destroy)
                .await
        );

        // Test different user
        assert!(
            !service
                .check_permission("other-user", "test-app", &Permission::View)
                .await
        );

        println!("✅ Basic authorization flow test passed");
    }

    #[tokio::test]
    async fn test_multi_group_app() {
        let (service, _temp_dir) = create_test_service().await;

        // Create multiple groups
        service
            .create_group("frontend", "Frontend applications")
            .await
            .unwrap();
        service
            .create_group("backend", "Backend services")
            .await
            .unwrap();

        // App belongs to multiple groups
        service
            .set_app_groups(
                "full-stack-app",
                vec!["frontend".to_string(), "backend".to_string()],
            )
            .await
            .unwrap();

        // User has access to frontend only
        service
            .assign_user_role("frontend-dev", "developer", vec!["frontend".to_string()])
            .await
            .unwrap();

        // User has access to backend only
        service
            .assign_user_role("backend-dev", "developer", vec!["backend".to_string()])
            .await
            .unwrap();

        // Both should be able to access the app
        assert!(
            service
                .check_permission("frontend-dev", "full-stack-app", &Permission::View)
                .await
        );
        assert!(
            service
                .check_permission("backend-dev", "full-stack-app", &Permission::View)
                .await
        );

        println!("✅ Multi-group app test passed");
    }

    #[tokio::test]
    async fn test_admin_permissions() {
        let (service, _temp_dir) = create_test_service().await;

        // Create group and app
        service
            .create_group("admin-group", "Admin test group")
            .await
            .unwrap();
        service
            .set_app_groups("admin-app", vec!["admin-group".to_string()])
            .await
            .unwrap();

        // Assign admin role
        service
            .assign_user_role("admin-user", "admin", vec!["admin-group".to_string()])
            .await
            .unwrap();

        // Admin should have all permissions
        assert!(
            service
                .check_permission("admin-user", "admin-app", &Permission::View)
                .await
        );
        assert!(
            service
                .check_permission("admin-user", "admin-app", &Permission::Manage)
                .await
        );
        assert!(
            service
                .check_permission("admin-user", "admin-app", &Permission::Shell)
                .await
        );
        assert!(
            service
                .check_permission("admin-user", "admin-app", &Permission::Destroy)
                .await
        );

        println!("✅ Admin permissions test passed");
    }
}
