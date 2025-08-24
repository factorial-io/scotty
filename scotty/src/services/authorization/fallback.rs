use casbin::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::casbin::CasbinManager;
use super::service::AuthorizationService;
use super::types::{
    Assignment, AuthConfig, GroupConfig, Permission, PermissionOrWildcard, RoleConfig,
};

/// Fallback authorization service creation
pub struct FallbackService;

impl FallbackService {
    /// Create a fallback authorization service with minimal configuration
    pub async fn create_fallback_service(
        legacy_access_token: Option<String>,
    ) -> AuthorizationService {
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
        let mut config = Self::create_minimal_config();

        // Add legacy access token if provided
        if let Some(token) = legacy_access_token {
            let user_id = AuthorizationService::format_user_id("", Some(&token));
            config.assignments.insert(
                user_id,
                vec![Assignment {
                    role: "admin".to_string(),
                    groups: vec!["default".to_string()],
                }],
            );
        }

        // Assign all apps to default group and sync policies
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
        }
    }
}
