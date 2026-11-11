//! Characterization of policy changes at runtime (admin API calls and app
//! scopes) and of loading the policy at startup, as they behave today.
//!
//! Each test works on its own temp policy directory. Assertions marked
//! `// CHANGES:` pin behavior that is expected to change in a planned rewrite
//! of the authorization code; the marker says to what.

use std::path::Path;

use scotty::services::authorization::types::PermissionOrWildcard;
use scotty::services::authorization::Permission;
use scotty::services::AuthorizationService;

const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
  beta: { description: Beta, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: Admin }
  dev: { permissions: [view, manage, create], description: Developer }
assignments:
  root@example.com:
    - { role: admin, scopes: ['*'] }
  ann@example.com:
    - { role: dev, scopes: [alpha] }
  bob@example.com:
    - { role: dev, scopes: [beta] }
"#;

/// A directory holding the shipped `model.conf` and, if given, `policy.yaml`.
fn policy_dir(policy: Option<&str>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::copy("../config/casbin/model.conf", dir.path().join("model.conf")).unwrap();
    if let Some(policy) = policy {
        std::fs::write(dir.path().join("policy.yaml"), policy).unwrap();
    }
    dir
}

async fn load(dir: &Path) -> anyhow::Result<AuthorizationService> {
    AuthorizationService::new(dir.to_str().unwrap()).await
}

fn scopes(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}

// --- Runtime changes ----------------------------------------------------------

#[tokio::test]
async fn new_scope_and_wildcard_scope_assignment() {
    let dir = policy_dir(Some(POLICY));
    let auth = load(dir.path()).await.unwrap();
    auth.create_scope("qa", "QA").await.unwrap();
    auth.set_app_scopes("qa-app", scopes(&["qa"]))
        .await
        .unwrap();

    // CHANGES: per-app checks miss the new scope for `*` assignments until a
    // restart (the casbin index expanded `*` at load time) -> allowed.
    assert!(
        !auth
            .check_permission("root@example.com", "qa-app", &Permission::Manage)
            .await
    );
    // Scope checks and listings expand `*` on every call and see it at once.
    assert!(
        auth.check_permission_in_scopes("root@example.com", &scopes(&["qa"]), &Permission::Manage)
            .await
    );
    assert!(
        auth.check_create_in_scopes("root@example.com", &scopes(&["qa"]), None)
            .await
    );
    let listed = auth.get_user_permissions("root@example.com").await;
    assert_eq!(listed.get("qa"), Some(&vec!["*".to_string()]));

    // After a restart (app scopes come from the app scan again) it is allowed.
    let restarted = load(dir.path()).await.unwrap();
    restarted
        .set_app_scopes("qa-app", scopes(&["qa"]))
        .await
        .unwrap();
    assert!(
        restarted
            .check_permission("root@example.com", "qa-app", &Permission::Manage)
            .await
    );
}

#[tokio::test]
async fn assign_role_takes_effect_at_once() {
    let dir = policy_dir(Some(POLICY));
    let auth = load(dir.path()).await.unwrap();
    auth.set_app_scopes("app-a", scopes(&["alpha"]))
        .await
        .unwrap();
    let user = "new@other.test";
    assert!(
        !auth
            .check_permission(user, "app-a", &Permission::Manage)
            .await
    );

    auth.assign_user_role(user, "dev", scopes(&["alpha"]))
        .await
        .unwrap();
    assert!(
        auth.check_permission(user, "app-a", &Permission::Manage)
            .await
    );
    assert!(
        auth.check_permission_in_scopes(user, &scopes(&["alpha"]), &Permission::Manage)
            .await
    );
    assert!(
        !auth
            .check_permission(user, "app-a", &Permission::Destroy)
            .await
    );

    // A domain key reaches every user of the domain in per-app checks too.
    auth.assign_user_role("@team.test", "dev", scopes(&["alpha"]))
        .await
        .unwrap();
    assert!(
        auth.check_permission("Kim@Team.test", "app-a", &Permission::View)
            .await
    );

    // Assigning the same role and scopes twice keeps one assignment.
    auth.assign_user_role(user, "dev", scopes(&["alpha"]))
        .await
        .unwrap();
    assert_eq!(auth.list_assignments().await[user].len(), 1);

    let err = auth
        .assign_user_role(user, "devloper", scopes(&["alpha"]))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("devloper"), "{err}");

    // CHANGES: an undefined scope is accepted today -> refused, naming it.
    auth.assign_user_role(user, "dev", scopes(&["stagin"]))
        .await
        .unwrap();
    assert!(auth.list_assignments().await[user]
        .iter()
        .any(|a| a.scopes == ["stagin"]));
}

#[tokio::test]
async fn create_role_and_duplicates() {
    let dir = policy_dir(Some(POLICY));
    let auth = load(dir.path()).await.unwrap();
    auth.set_app_scopes("app-a", scopes(&["alpha"]))
        .await
        .unwrap();

    auth.create_role(
        "auditor",
        vec![PermissionOrWildcard::Permission(Permission::Logs)],
        "Audit",
    )
    .await
    .unwrap();
    auth.assign_user_role("eve@other.test", "auditor", scopes(&["alpha"]))
        .await
        .unwrap();
    assert!(
        auth.check_permission("eve@other.test", "app-a", &Permission::Logs)
            .await
    );
    assert!(
        !auth
            .check_permission("eve@other.test", "app-a", &Permission::View)
            .await
    );

    assert!(auth.create_role("auditor", vec![], "again").await.is_err());
    assert!(auth.create_scope("alpha", "again").await.is_err());

    // The written policy loads again and keeps the changes.
    let reloaded = load(dir.path()).await.unwrap();
    assert!(reloaded
        .list_roles()
        .await
        .iter()
        .any(|(name, _)| name == "auditor"));
    assert!(reloaded
        .list_assignments()
        .await
        .contains_key("eve@other.test"));
}

#[tokio::test]
async fn set_app_scopes_moves_the_app() {
    let dir = policy_dir(Some(POLICY));
    let auth = load(dir.path()).await.unwrap();
    auth.set_app_scopes("app", scopes(&["alpha"]))
        .await
        .unwrap();
    // Ask once before the move, so a stale cached answer would show.
    assert!(
        auth.check_permission("ann@example.com", "app", &Permission::Manage)
            .await
    );
    assert!(
        !auth
            .check_permission("bob@example.com", "app", &Permission::Manage)
            .await
    );

    auth.set_app_scopes("app", scopes(&["beta"])).await.unwrap();
    assert!(
        !auth
            .check_permission("ann@example.com", "app", &Permission::Manage)
            .await
    );
    assert!(
        auth.check_permission("bob@example.com", "app", &Permission::Manage)
            .await
    );

    auth.set_app_scopes("app", vec![]).await.unwrap();
    assert!(
        !auth
            .check_permission("root@example.com", "app", &Permission::View)
            .await
    );

    // An undefined scope is not checked here (callers validate first).
    auth.set_app_scopes("app", scopes(&["nope"])).await.unwrap();
    assert!(
        !auth
            .check_permission("root@example.com", "app", &Permission::View)
            .await
    );

    // App scopes are not written to policy.yaml.
    let written = std::fs::read_to_string(dir.path().join("policy.yaml")).unwrap();
    assert!(!written.contains("apps:"), "{written}");
}

#[tokio::test]
async fn failed_save_keeps_the_change_in_memory() {
    let dir = policy_dir(Some(POLICY));
    let auth = load(dir.path()).await.unwrap();
    auth.set_app_scopes("app-a", scopes(&["alpha"]))
        .await
        .unwrap();
    // Make writing policy.yaml fail.
    let path = dir.path().join("policy.yaml");
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    assert!(auth.create_scope("qa", "QA").await.is_err());
    // CHANGES: the scope exists although the request failed -> unchanged.
    assert!(auth.get_scopes().await.contains(&"qa".to_string()));

    let user = "sam@other.test";
    assert!(auth
        .assign_user_role(user, "dev", scopes(&["alpha"]))
        .await
        .is_err());
    // CHANGES: the failed assignment is in effect -> not allowed.
    assert!(
        auth.check_permission(user, "app-a", &Permission::Manage)
            .await
    );
}

// --- Loading ------------------------------------------------------------------

#[tokio::test]
async fn missing_policy_is_seeded_from_the_example() {
    let dir = policy_dir(None);
    let example = std::fs::read_to_string("../config/casbin/policy.yaml.example").unwrap();
    std::fs::write(dir.path().join("policy.yaml.example"), &example).unwrap();

    let auth = load(dir.path()).await.unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join("policy.yaml")).unwrap(),
        example
    );
    assert!(auth.is_enabled().await);
    assert!(auth
        .list_roles()
        .await
        .iter()
        .any(|(name, _)| name == "artifact_publisher"));
}

#[tokio::test]
async fn neither_policy_nor_example_gives_defaults() {
    let dir = policy_dir(None);
    let auth = load(dir.path()).await.unwrap();

    assert_eq!(auth.get_scopes().await, ["default"]);
    let mut roles: Vec<String> = auth.list_roles().await.into_iter().map(|r| r.0).collect();
    roles.sort();
    assert_eq!(roles, ["admin", "developer", "operator", "viewer"]);
    // No assignments, so the authorization middleware refuses every request.
    assert!(!auth.is_enabled().await);
    // The defaults are not written to disk.
    assert!(!dir.path().join("policy.yaml").exists());
}

async fn load_error(policy: &str) -> String {
    let dir = policy_dir(Some(policy));
    format!("{:#}", load(dir.path()).await.unwrap_err())
}

#[tokio::test]
async fn broken_policies_fail_to_load() {
    let err = load_error("scopes: [unclosed").await;
    assert!(
        err.contains("Failed to parse authorization config"),
        "{err}"
    );

    let err = load_error("roles: {}\nassignments: {}\n").await;
    assert!(err.contains("missing field `scopes`"), "{err}");

    let err = load_error(&POLICY.replace("[view, manage, create]", "[view, veiw]")).await;
    assert!(err.contains("unknown permission 'veiw'"), "{err}");
}

#[tokio::test]
async fn undefined_role_or_scope_in_an_assignment_loads() {
    // CHANGES: both load today -> fail, naming the user pattern and the key.
    let policy = POLICY.replace(
        "  bob@example.com:\n    - { role: dev, scopes: [beta] }",
        "  bob@example.com:\n    - { role: devloper, scopes: [beta] }\n    - { role: dev, scopes: [stagin] }",
    );
    let dir = policy_dir(Some(&policy));
    let auth = load(dir.path()).await.unwrap();
    // The undefined role grants nothing; the undefined scope is listed.
    auth.set_app_scopes("app-b", scopes(&["beta"]))
        .await
        .unwrap();
    assert!(
        !auth
            .check_permission("bob@example.com", "app-b", &Permission::View)
            .await
    );
    let listed = auth.get_user_permissions("bob@example.com").await;
    assert_eq!(listed.keys().collect::<Vec<_>>(), ["stagin"]);
}

#[tokio::test]
async fn missing_model_fails_to_load() {
    let dir = policy_dir(Some(POLICY));
    std::fs::remove_file(dir.path().join("model.conf")).unwrap();
    // CHANGES: no model file is read any more -> loads.
    let err = format!("{:#}", load(dir.path()).await.unwrap_err());
    assert!(err.contains("Failed to load Casbin model"), "{err}");
}
