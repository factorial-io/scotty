use casbin::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::casbin::{register_user_match_function, CasbinManager};
use super::service::AuthorizationService;
use super::types::{
    Assignment, AuthConfig, Permission, PermissionOrWildcard, RoleConfig, ScopeConfig,
};

/// Fallback authorization service creation
pub struct FallbackService;

impl FallbackService {
    /// Create a fallback authorization service with minimal configuration
    pub async fn create_fallback_service(
        legacy_access_token: Option<String>,
    ) -> AuthorizationService {
        // Create a minimal Casbin model in memory
        // Uses user_match() custom function for domain/wildcard matching
        let model_text = r#"
[request_definition]
r = sub, app, act

[policy_definition]
p = sub, scope, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = user_match(r.sub, p.sub) && g2(r.app, p.scope) && r.act == p.act
"#;

        let m = DefaultModel::from_str(model_text)
            .await
            .expect("Failed to create fallback Casbin model");

        let a = MemoryAdapter::default();
        let mut enforcer = CachedEnforcer::new(m, a)
            .await
            .expect("Failed to create fallback Casbin enforcer");

        // Register custom user_match function for domain/wildcard matching
        register_user_match_function(&mut enforcer);

        // Create default configuration with everyone having access to "default" scope
        let mut config = Self::create_minimal_config();

        // Add test app mappings to default scope for fallback service
        config
            .apps
            .insert("test-app".to_string(), vec!["default".to_string()]);
        config
            .apps
            .insert("nonexistent-app".to_string(), vec!["default".to_string()]);

        // Add legacy access token if provided
        if let Some(token) = legacy_access_token {
            let user_id = AuthorizationService::format_user_id("", Some(&token));
            config.assignments.insert(
                user_id,
                vec![Assignment {
                    role: "admin".to_string(),
                    scopes: vec!["default".to_string()],
                }],
            );
        }

        // Assign all apps to default scope and sync policies
        CasbinManager::sync_policies_to_casbin(&mut enforcer, &config)
            .await
            .expect("Failed to sync fallback policies to Casbin");

        info!("Fallback authorization service created with default configuration");

        AuthorizationService::new_from_components(
            Arc::new(RwLock::new(enforcer)),
            Arc::new(RwLock::new(config)),
            "fallback/policy.yaml".to_string(), // Placeholder path for fallback
        )
    }

    /// Create minimal configuration for fallback service
    fn create_minimal_config() -> AuthConfig {
        AuthConfig {
            scopes: HashMap::from([(
                "default".to_string(),
                ScopeConfig {
                    description: "Default scope for all users".to_string(),
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
        }
    }
}
