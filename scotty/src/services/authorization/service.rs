use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use casbin::CachedEnforcer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

use super::config::ConfigManager;
use super::decision::{Action, AppScopes, Decision, Target};
use super::fallback::FallbackService;
use super::policy::{Grants, Policy, PolicyError};
use super::types::{
    Assignment, AuthConfig, Permission, PermissionOrWildcard, RoleConfig, ScopeConfig,
};

/// Authorization service: one decision point over immutable snapshots.
///
/// Readers load the current [`Policy`] and app scopes without locking.
/// Writers serialize on `write`, compile a changed copy of the policy, save
/// it, and only then swap it in, so a failed change leaves the old snapshot
/// in effect.
pub struct AuthorizationService {
    policy: ArcSwap<Policy>,
    /// App -> scopes, maintained by the app scan; never written to disk.
    app_scopes: ArcSwap<AppScopes>,
    write: Mutex<()>,
    config_path: String,
}

impl AuthorizationService {
    /// Load and validate `policy.yaml` from `config_dir`.
    pub async fn new(config_dir: &str) -> Result<Self> {
        let policy_path = format!("{}/policy.yaml", config_dir);
        let config = ConfigManager::load_config(&policy_path).await?;
        let service = Self::from_config(config, policy_path)?;
        let policy = service.policy.load();
        info!(
            "Authorization service initialized with {} scopes, {} roles",
            policy.config().scopes.len(),
            policy.config().roles.len()
        );
        drop(policy);
        Ok(service)
    }

    /// A service over `config`, saving changes to `config_path`. App scopes
    /// in `config.apps` seed the app-scope snapshot.
    pub(crate) fn from_config(mut config: AuthConfig, config_path: String) -> Result<Self> {
        let apps = std::mem::take(&mut config.apps);
        let policy = Policy::compile(config).context("Invalid authorization policy")?;
        Ok(Self {
            policy: ArcSwap::from_pointee(policy),
            app_scopes: ArcSwap::from_pointee(apps),
            write: Mutex::new(()),
            config_path,
        })
    }

    /// Create a fallback authorization service with minimal configuration
    pub async fn create_fallback_service(legacy_access_token: Option<String>) -> Self {
        FallbackService::create_fallback_service(legacy_access_token).await
    }

    /// Create a service from a configuration. The enforcer is ignored; the
    /// signature is kept for existing callers until Casbin is removed.
    ///
    /// # Panics
    ///
    /// If `config` is locked or does not compile.
    #[allow(dead_code)] // the binary doesn't use it; integration tests do
    pub fn new_from_components(
        _enforcer: Arc<RwLock<CachedEnforcer>>,
        config: Arc<RwLock<AuthConfig>>,
        config_path: String,
    ) -> Self {
        let config = config
            .try_read()
            .expect("authorization config is not locked")
            .clone();
        Self::from_config(config, config_path).expect("authorization config compiles")
    }

    /// The `policy.yaml` changes are saved to.
    pub fn config_path(&self) -> &str {
        &self.config_path
    }

    /// The decision for `user` doing `action` on `target`.
    pub fn decide(&self, user: &str, action: Action<'_>, target: Target<'_>) -> Decision {
        let decision = self
            .policy
            .load()
            .decide(&self.app_scopes.load(), user, action, target);
        debug!(
            ?action,
            ?target,
            ?decision,
            "Authorization decision for {user}"
        );
        decision
    }

    /// The one enforcement call: `Ok` if allowed, otherwise 403 with the
    /// decision's reason.
    pub fn require(
        &self,
        user: &str,
        action: Action<'_>,
        target: Target<'_>,
    ) -> Result<(), crate::api::error::AppError> {
        match self.decide(user, action, target) {
            Decision::Allow => Ok(()),
            Decision::Deny(reason) => {
                warn!(?action, ?target, "Access denied for {user}: {reason}");
                Err(crate::api::error::AppError::ScopeAccessDenied(reason))
            }
        }
    }

    /// The current app -> scopes snapshot.
    pub fn app_scopes(&self) -> Arc<AppScopes> {
        self.app_scopes.load_full()
    }

    /// Everything `user` holds, by scope.
    pub fn grants_for(&self, user: &str) -> Grants {
        self.policy.load().grants_for(user)
    }

    /// Whether `user` may see `task` (its details, updates and output).
    ///
    /// The user who started the task always may, which covers app creation
    /// before the new app is assigned to a scope. A task without an app needs
    /// `admin_read`. A task of an app needs `admin_read` on that app (in one
    /// of its scopes), or the permission the task requires: for an action run
    /// the permission the action required, so `view` alone doesn't reveal its
    /// output, and `view` for any other task. An app without scopes falls
    /// back to `admin_read` in any scope.
    pub async fn can_view_task(
        &self,
        user: &str,
        task: &scotty_core::tasks::task_details::TaskDetails,
    ) -> bool {
        if task.requested_by.as_deref() == Some(user) {
            return true;
        }
        let grants = self.grants_for(user);
        let apps = self.app_scopes.load();
        let allows = |permission: Permission, target| {
            grants.decide(&apps, permission.into(), target).is_allowed()
        };
        let Some(app) = &task.app_name else {
            return allows(Permission::AdminRead, Target::Global);
        };
        let required = match task.action_permission.as_deref() {
            None => Some(Permission::View),
            // Fail closed: an unknown permission is never granted.
            Some(name) => Permission::from_str(name),
        };
        if required.is_some_and(|p| allows(p, Target::App(app))) {
            return true;
        }
        // An app without scopes (e.g. a create that failed before its scope
        // was written) is in no one's scope; admins see it globally.
        if apps
            .get(app.as_str())
            .is_none_or(|scopes| scopes.is_empty())
        {
            return allows(Permission::AdminRead, Target::Global);
        }
        allows(Permission::AdminRead, Target::App(app))
    }

    /// Format user identifier for authorization checks
    pub fn format_user_id(email: &str, token: Option<&str>) -> String {
        if let Some(token) = token {
            format!("bearer:{}", token)
        } else {
            email.to_string()
        }
    }

    /// The user id authorization checks use for `user`: the email of OAuth
    /// users, `identifier:<name>` for bearer tokens.
    pub fn get_user_id_for_authorization(user: &crate::api::basic_auth::CurrentUser) -> String {
        user.email.clone()
    }

    /// Validate domain assignment pattern (e.g., "@factorial.io")
    /// Returns Ok(()) if valid or not a domain pattern, Err with message if invalid
    pub fn validate_domain_assignment(user_id: &str) -> Result<()> {
        // Not a domain pattern - skip validation
        let Some(domain) = user_id.strip_prefix('@') else {
            return Ok(());
        };

        if domain.is_empty() {
            anyhow::bail!("Domain cannot be empty (just '@')");
        }
        if !domain.contains('.') {
            anyhow::bail!(
                "Invalid domain '{}': must contain a dot (e.g., '@example.com')",
                user_id
            );
        }
        if domain.contains('@') {
            anyhow::bail!("Invalid domain '{}': contains '@' character", user_id);
        }
        Ok(())
    }

    /// Check if authorization is enabled (has any assignments)
    pub async fn is_enabled(&self) -> bool {
        !self.policy.load().config().assignments.is_empty()
    }

    /// Look up user information by bearer token
    #[allow(dead_code)]
    pub async fn get_user_by_token(&self, token: &str) -> Option<String> {
        let token_user_id = Self::format_user_id("", Some(token));
        self.get_user_by_identifier(&token_user_id).await
    }

    /// Look up user information by identifier (new format: identifier:admin)
    pub async fn get_user_by_identifier(&self, identifier_user_id: &str) -> Option<String> {
        self.policy
            .load()
            .config()
            .assignments
            .contains_key(identifier_user_id)
            .then(|| identifier_user_id.to_string())
    }

    /// Validate that all specified scopes exist in the authorization system
    /// Returns Ok(()) if all scopes exist, or Err with missing scopes if not
    pub async fn validate_scopes(&self, scopes: &[String]) -> Result<(), Vec<String>> {
        let policy = self.policy.load();
        let missing_scopes: Vec<String> = scopes
            .iter()
            .filter(|scope| !policy.config().scopes.contains_key(*scope))
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
    ) -> Vec<crate::api::rest::handlers::scopes::list::ScopeInfo> {
        let policy = self.policy.load();
        policy
            .grants_for(user)
            .scopes()
            .iter()
            .map(
                |(scope, grant)| crate::api::rest::handlers::scopes::list::ScopeInfo {
                    name: scope.clone(),
                    description: policy
                        .config()
                        .scopes
                        .get(scope)
                        .map(|s| s.description.clone())
                        .unwrap_or_else(|| format!("Scope: {}", scope)),
                    permissions: grant.listed.clone(),
                },
            )
            .collect()
    }

    /// Get user's effective permissions, by scope, as written in the policy
    pub async fn get_user_permissions(&self, user: &str) -> HashMap<String, Vec<String>> {
        self.grants_for(user)
            .scopes()
            .iter()
            .map(|(scope, grant)| (scope.clone(), grant.listed.clone()))
            .collect()
    }

    /// Assign an app to scopes. Takes effect for every following check.
    /// Note: Caller should validate scopes exist using validate_scopes() before calling this
    pub async fn set_app_scopes(&self, app: &str, scopes: Vec<String>) -> Result<()> {
        let _write = self.write.lock().await;
        let mut apps = AppScopes::clone(&self.app_scopes.load());
        apps.insert(app.to_string(), scopes);
        self.app_scopes.store(Arc::new(apps));
        Ok(())
    }

    /// Create a new scope
    pub async fn create_scope(&self, name: &str, description: &str) -> Result<()> {
        self.update(|config| {
            if config.scopes.contains_key(name) {
                return Err(PolicyError::DuplicateScope(name.to_string()));
            }
            config.scopes.insert(
                name.to_string(),
                ScopeConfig {
                    description: description.to_string(),
                    created_at: chrono::Utc::now(),
                },
            );
            Ok(())
        })
        .await?;
        info!("Created scope '{}'", name);
        Ok(())
    }

    /// Get all scopes
    pub async fn list_scopes(&self) -> Vec<(String, ScopeConfig)> {
        self.policy
            .load()
            .config()
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
        self.update(|config| {
            if config.roles.contains_key(name) {
                return Err(PolicyError::DuplicateRole(name.to_string()));
            }
            config.roles.insert(
                name.to_string(),
                RoleConfig {
                    permissions,
                    description: description.to_string(),
                },
            );
            Ok(())
        })
        .await?;
        info!("Created role '{}'", name);
        Ok(())
    }

    /// Assign role to user for specific scopes. Assigning the same role and
    /// scopes again changes nothing.
    pub async fn assign_user_role(
        &self,
        user: &str,
        role: &str,
        scopes: Vec<String>,
    ) -> Result<()> {
        self.update(|config| {
            let assignments = config.assignments.entry(user.to_string()).or_default();
            if !assignments
                .iter()
                .any(|a| a.role == role && a.scopes == scopes)
            {
                assignments.push(Assignment {
                    role: role.to_string(),
                    scopes,
                });
            }
            Ok(())
        })
        .await?;
        info!("Assigned role '{}' to user '{}'", role, user);
        Ok(())
    }

    /// Get all roles
    pub async fn list_roles(&self) -> Vec<(String, RoleConfig)> {
        self.policy
            .load()
            .config()
            .roles
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Get all assignments
    pub async fn list_assignments(&self) -> HashMap<String, Vec<Assignment>> {
        self.policy.load().config().assignments.clone()
    }

    /// Apply `change` to a copy of the policy, validate and save the result,
    /// then swap it in. Any error leaves the current policy in effect.
    async fn update(
        &self,
        change: impl FnOnce(&mut AuthConfig) -> Result<(), PolicyError>,
    ) -> Result<()> {
        let _write = self.write.lock().await;
        let mut config = self.policy.load().config().clone();
        change(&mut config)?;
        let policy = Policy::compile(config)?;
        // The fallback service lives in memory only.
        if !self.config_path.starts_with("fallback/") {
            ConfigManager::save_config(policy.config(), self.config_path()).await?;
        }
        self.policy.store(Arc::new(policy));
        Ok(())
    }
}

/// Boolean shorthands over [`AuthorizationService::decide`]. The server
/// itself calls `require`/`decide`; these remain for tests.
#[allow(dead_code)]
impl AuthorizationService {
    /// Check if a user has permission to perform an action on an app
    pub async fn check_permission(&self, user: &str, app: &str, action: &Permission) -> bool {
        self.decide(user, (*action).into(), Target::App(app))
            .is_allowed()
    }

    /// Check if a user has a permission in any of their scopes (not tied to
    /// a specific app).
    pub async fn check_global_permission(&self, user: &str, action: &Permission) -> bool {
        self.decide(user, (*action).into(), Target::Global)
            .is_allowed()
    }

    /// Check if a user has permission in any of the specified scopes
    pub async fn check_permission_in_scopes(
        &self,
        user: &str,
        scopes: &[String],
        action: &Permission,
    ) -> bool {
        self.decide(user, (*action).into(), Target::AnyOf(scopes))
            .is_allowed()
    }

    /// Check whether a user may create an app in any of `scopes`.
    ///
    /// Plain `create` allows everything. `create:<blueprint>` allows only
    /// `blueprint`, which the caller passes as `Some` only when the request
    /// uses a templated blueprint and carries no compose file of its own.
    pub async fn check_create_in_scopes(
        &self,
        user: &str,
        scopes: &[String],
        blueprint: Option<&str>,
    ) -> bool {
        self.decide(user, Action::create(blueprint), Target::AnyOf(scopes))
            .is_allowed()
    }

    /// Like [`Self::check_create_in_scopes`], but the user needs the right
    /// in every one of `scopes`, not just one. No scopes means no right.
    pub async fn check_create_in_every_scope(
        &self,
        user: &str,
        scopes: &[String],
        blueprint: Option<&str>,
    ) -> bool {
        self.decide(user, Action::create(blueprint), Target::EveryOf(scopes))
            .is_allowed()
    }

    /// Get all available scopes defined in the authorization configuration
    pub async fn get_scopes(&self) -> Vec<String> {
        self.policy.load().config().scopes.keys().cloned().collect()
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
mod domain_tests {
    use super::*;

    #[test]
    fn test_validate_domain_assignment() {
        // Valid domain patterns
        assert!(AuthorizationService::validate_domain_assignment("@factorial.io").is_ok());
        assert!(AuthorizationService::validate_domain_assignment("@sub.factorial.io").is_ok());
        assert!(AuthorizationService::validate_domain_assignment("@example.co.uk").is_ok());

        // Non-domain patterns (should pass - not validated)
        assert!(AuthorizationService::validate_domain_assignment("user@factorial.io").is_ok());
        assert!(AuthorizationService::validate_domain_assignment("identifier:admin").is_ok());
        assert!(AuthorizationService::validate_domain_assignment("*").is_ok());

        // Invalid domain patterns
        assert!(AuthorizationService::validate_domain_assignment("@").is_err()); // Empty domain
        assert!(AuthorizationService::validate_domain_assignment("@factorial").is_err()); // No dot
        assert!(AuthorizationService::validate_domain_assignment("@@factorial.io").is_err()); // Extra @
        assert!(AuthorizationService::validate_domain_assignment("@facto@rial.io").is_err());
        // @ in domain
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: Admin }
assignments:
  root@example.com:
    - { role: admin, scopes: ['*'] }
"#;

    async fn service() -> (Arc<AuthorizationService>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("policy.yaml"), POLICY).unwrap();
        let auth = AuthorizationService::new(dir.path().to_str().unwrap())
            .await
            .unwrap();
        (Arc::new(auth), dir)
    }

    #[tokio::test]
    async fn new_scope_reaches_wildcard_assignments_at_once() {
        let (auth, _dir) = service().await;
        auth.create_scope("qa", "QA").await.unwrap();
        auth.set_app_scopes("qa-app", vec!["qa".into()])
            .await
            .unwrap();
        assert!(
            auth.check_permission("root@example.com", "qa-app", &Permission::Manage)
                .await
        );
    }

    #[tokio::test]
    async fn failed_save_changes_nothing() {
        let (auth, dir) = service().await;
        let path = dir.path().join("policy.yaml");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();

        assert!(auth.create_scope("qa", "QA").await.is_err());
        assert!(!auth.get_scopes().await.contains(&"qa".to_string()));
        assert!(auth
            .assign_user_role("sam@example.com", "admin", vec!["alpha".into()])
            .await
            .is_err());
        assert!(
            !auth
                .check_global_permission("sam@example.com", &Permission::View)
                .await
        );
        // The temp file of the failed write is cleaned up.
        assert!(!dir.path().join("policy.yaml.tmp").exists());
    }

    #[tokio::test]
    async fn invalid_change_is_refused_and_changes_nothing() {
        let (auth, dir) = service().await;
        let before = std::fs::read_to_string(dir.path().join("policy.yaml")).unwrap();
        let err = auth
            .assign_user_role("sam@example.com", "admin", vec!["stagin".into()])
            .await
            .unwrap_err();
        assert!(matches!(
            err.downcast_ref::<PolicyError>(),
            Some(PolicyError::UndefinedScope { scope, .. }) if scope == "stagin"
        ));
        assert!(!auth
            .list_assignments()
            .await
            .contains_key("sam@example.com"));
        assert_eq!(
            std::fs::read_to_string(dir.path().join("policy.yaml")).unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn concurrent_writers_all_apply() {
        let (auth, dir) = service().await;
        let writers = (0..16).map(|i| {
            let auth = auth.clone();
            tokio::spawn(async move {
                auth.create_scope(&format!("scope-{i}"), "concurrent")
                    .await
                    .unwrap();
                auth.set_app_scopes(&format!("app-{i}"), vec![format!("scope-{i}")])
                    .await
                    .unwrap();
            })
        });
        for writer in writers.collect::<Vec<_>>() {
            writer.await.unwrap();
        }
        assert_eq!(auth.get_scopes().await.len(), 17);
        for i in 0..16 {
            assert!(
                auth.check_permission("root@example.com", &format!("app-{i}"), &Permission::View)
                    .await
            );
        }
        // Every change reached the file, too.
        let reloaded = AuthorizationService::new(dir.path().to_str().unwrap())
            .await
            .unwrap();
        assert_eq!(reloaded.get_scopes().await.len(), 17);
    }

    #[tokio::test]
    async fn readers_keep_their_snapshot_while_a_change_lands() {
        let (auth, _dir) = service().await;
        let before = auth.policy.load_full();
        auth.create_scope("qa", "QA").await.unwrap();
        assert!(!before.config().scopes.contains_key("qa"));
        assert!(auth.policy.load().config().scopes.contains_key("qa"));
    }

    #[tokio::test]
    async fn require_maps_a_deny_to_403_with_the_reason() {
        let (auth, _dir) = service().await;
        assert!(auth
            .require(
                "root@example.com",
                Permission::Manage.into(),
                Target::Global
            )
            .is_ok());
        let err = auth
            .require(
                "nobody@example.com",
                Permission::Manage.into(),
                Target::Global,
            )
            .unwrap_err();
        assert!(matches!(
            &err,
            crate::api::error::AppError::ScopeAccessDenied(reason)
                if reason == "user lacks manage permission"
        ));
    }
}
