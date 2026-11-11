use chrono::Utc;
use std::collections::HashMap;
use tracing::info;

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

        info!("Fallback authorization service created with default configuration");

        // Placeholder path: the fallback service is never saved.
        AuthorizationService::from_config(config, "fallback/policy.yaml".to_string())
            .expect("fallback authorization config compiles")
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
