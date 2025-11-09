use scotty::services::authorization::types::{
    Assignment, AuthConfig, PermissionOrWildcard, RoleConfig, ScopeConfig,
};
use scotty::services::authorization::AuthorizationService;
use std::collections::HashMap;

/// Test domain assignment resolution with exact email match
#[tokio::test]
async fn test_exact_email_takes_precedence_over_domain() {
    let config = create_test_config_with_domain_assignments();
    let service = create_test_service(config).await;

    // User with exact email assignment should get exact + wildcard (NOT domain)
    let assignments = service.get_user_permissions("admin@factorial.io").await;

    // Should have permissions from "admin" role (exact match) and "viewer" role (wildcard)
    assert!(assignments.contains_key("admin-scope"));
    assert!(assignments.contains_key("default"));

    // Should NOT have permissions from domain match
    assert!(!assignments.contains_key("dev-scope"));
}

/// Test domain assignment resolution for users without exact match
#[tokio::test]
async fn test_domain_match_fallback() {
    let config = create_test_config_with_domain_assignments();
    let service = create_test_service(config).await;

    // User WITHOUT exact email assignment should get domain + wildcard
    let assignments = service.get_user_permissions("developer@factorial.io").await;

    // Should have permissions from "developer" role (domain match) and "viewer" role (wildcard)
    assert!(assignments.contains_key("dev-scope"));
    assert!(assignments.contains_key("default"));

    // Should NOT have exact match permissions
    assert!(!assignments.contains_key("admin-scope"));
}

/// Test wildcard-only assignment for non-matching domains
#[tokio::test]
async fn test_wildcard_only_for_unmatched_domains() {
    let config = create_test_config_with_domain_assignments();
    let service = create_test_service(config).await;

    // User from different domain should only get wildcard
    let assignments = service.get_user_permissions("user@other.com").await;

    // Should ONLY have wildcard permissions
    assert_eq!(assignments.len(), 1);
    assert!(assignments.contains_key("default"));
}

/// Test global permission check with domain assignments
#[tokio::test]
async fn test_global_permission_with_domain_assignment() {
    let config = create_test_config_with_domain_assignments();
    let service = create_test_service(config).await;

    // User with domain match should have permissions from their domain role
    let has_create = service
        .check_global_permission(
            "developer@factorial.io",
            &scotty::services::authorization::types::Permission::Create,
        )
        .await;

    assert!(has_create);

    // User from other domain should not have create permission (only view from wildcard)
    let has_create = service
        .check_global_permission(
            "user@other.com",
            &scotty::services::authorization::types::Permission::Create,
        )
        .await;

    assert!(!has_create);
}

/// Test domain assignment validation - valid patterns
#[test]
fn test_domain_validation_valid_patterns() {
    // Valid domain patterns
    assert!(AuthorizationService::validate_domain_assignment("@factorial.io").is_ok());
    assert!(AuthorizationService::validate_domain_assignment("@sub.factorial.io").is_ok());
    assert!(AuthorizationService::validate_domain_assignment("@example.co.uk").is_ok());

    // Non-domain patterns should pass through
    assert!(AuthorizationService::validate_domain_assignment("user@factorial.io").is_ok());
    assert!(AuthorizationService::validate_domain_assignment("identifier:admin").is_ok());
    assert!(AuthorizationService::validate_domain_assignment("*").is_ok());
}

/// Test domain assignment validation - invalid patterns
#[test]
fn test_domain_validation_invalid_patterns() {
    // Invalid domain patterns should fail
    assert!(AuthorizationService::validate_domain_assignment("@").is_err());
    assert!(AuthorizationService::validate_domain_assignment("@factorial").is_err());
    assert!(AuthorizationService::validate_domain_assignment("@@factorial.io").is_err());
    assert!(AuthorizationService::validate_domain_assignment("@facto@rial.io").is_err());
}

/// Test assignment creation with domain pattern
#[tokio::test]
async fn test_assign_role_with_domain_pattern() {
    let config = AuthConfig {
        scopes: create_test_scopes(),
        roles: create_test_roles(),
        assignments: HashMap::new(),
        apps: HashMap::new(),
    };
    let service = create_test_service(config).await;

    // Should succeed with valid domain pattern
    let result = service
        .assign_user_role("@newdomain.com", "developer", vec!["dev-scope".to_string()])
        .await;

    assert!(result.is_ok());

    // Verify assignment was created
    let assignments = service.list_assignments().await;
    assert!(assignments.contains_key("@newdomain.com"));
}

/// Test that domain assignments work with scope-specific permission checks
#[tokio::test]
async fn test_scope_permission_check_with_domain() {
    let config = create_test_config_with_domain_assignments();
    let service = create_test_service(config).await;

    // User with domain match should have create permission in dev-scope
    let has_permission = service
        .check_permission_in_scopes(
            "developer@factorial.io",
            &["dev-scope".to_string()],
            &scotty::services::authorization::types::Permission::Create,
        )
        .await;

    assert!(has_permission);

    // Same user should NOT have create permission in admin-scope (not assigned)
    let has_permission = service
        .check_permission_in_scopes(
            "developer@factorial.io",
            &["admin-scope".to_string()],
            &scotty::services::authorization::types::Permission::Create,
        )
        .await;

    assert!(!has_permission);
}

// Helper functions

fn create_test_scopes() -> HashMap<String, ScopeConfig> {
    let mut scopes = HashMap::new();
    scopes.insert(
        "admin-scope".to_string(),
        ScopeConfig {
            description: "Admin scope".to_string(),
            created_at: chrono::Utc::now(),
        },
    );
    scopes.insert(
        "dev-scope".to_string(),
        ScopeConfig {
            description: "Development scope".to_string(),
            created_at: chrono::Utc::now(),
        },
    );
    scopes.insert(
        "default".to_string(),
        ScopeConfig {
            description: "Default scope".to_string(),
            created_at: chrono::Utc::now(),
        },
    );
    scopes
}

fn create_test_roles() -> HashMap<String, RoleConfig> {
    let mut roles = HashMap::new();

    roles.insert(
        "admin".to_string(),
        RoleConfig {
            permissions: vec![PermissionOrWildcard::Wildcard],
            description: "Admin role with all permissions".to_string(),
        },
    );

    roles.insert(
        "developer".to_string(),
        RoleConfig {
            permissions: vec![
                PermissionOrWildcard::Permission(
                    scotty::services::authorization::types::Permission::View,
                ),
                PermissionOrWildcard::Permission(
                    scotty::services::authorization::types::Permission::Manage,
                ),
                PermissionOrWildcard::Permission(
                    scotty::services::authorization::types::Permission::Create,
                ),
                PermissionOrWildcard::Permission(
                    scotty::services::authorization::types::Permission::Logs,
                ),
                PermissionOrWildcard::Permission(
                    scotty::services::authorization::types::Permission::Shell,
                ),
            ],
            description: "Developer role".to_string(),
        },
    );

    roles.insert(
        "viewer".to_string(),
        RoleConfig {
            permissions: vec![PermissionOrWildcard::Permission(
                scotty::services::authorization::types::Permission::View,
            )],
            description: "Viewer role (read-only)".to_string(),
        },
    );

    roles
}

fn create_test_config_with_domain_assignments() -> AuthConfig {
    let mut assignments = HashMap::new();

    // Exact email assignment
    assignments.insert(
        "admin@factorial.io".to_string(),
        vec![Assignment {
            role: "admin".to_string(),
            scopes: vec!["admin-scope".to_string()],
        }],
    );

    // Domain assignment
    assignments.insert(
        "@factorial.io".to_string(),
        vec![Assignment {
            role: "developer".to_string(),
            scopes: vec!["dev-scope".to_string()],
        }],
    );

    // Wildcard assignment
    assignments.insert(
        "*".to_string(),
        vec![Assignment {
            role: "viewer".to_string(),
            scopes: vec!["default".to_string()],
        }],
    );

    AuthConfig {
        scopes: create_test_scopes(),
        roles: create_test_roles(),
        assignments,
        apps: HashMap::new(),
    }
}

async fn create_test_service(config: AuthConfig) -> AuthorizationService {
    use casbin::prelude::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    // Create a minimal in-memory enforcer
    let m = DefaultModel::from_str(
        r#"
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
m = r.sub == p.sub && g2(r.app, p.scope) && r.act == p.act
"#,
    )
    .await
    .unwrap();

    let a = MemoryAdapter::default();
    let mut enforcer = casbin::CachedEnforcer::new(m, a).await.unwrap();

    // Sync policies to Casbin
    scotty::services::authorization::casbin::CasbinManager::sync_policies_to_casbin(
        &mut enforcer,
        &config,
    )
    .await
    .unwrap();

    AuthorizationService::new_from_components(
        Arc::new(RwLock::new(enforcer)),
        Arc::new(RwLock::new(config)),
        "test-config".to_string(),
    )
}
