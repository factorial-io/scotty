//! Permission decisions at the service level.
//!
//! Every check of `AuthorizationService` is called for a fixed set of user
//! kinds against the policy below, and the result is compared with the
//! expectations written out in the tables.
//!
//! One matching rule applies to every check and listing: a user's
//! assignments are the union of their exact entry (email or identifier),
//! their `@domain` entry and `*`. Rows marked `// CHANGED:` differ from the
//! behavior before that rule; the marker says how.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use scotty::services::authorization::Permission;
use scotty::services::AuthorizationService;

/// Scopes, roles and assignments of the matrix. App scopes are set at runtime
/// (see [`service`]), as the app scan does.
const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
  beta: { description: Beta, created_at: 2024-01-01T00:00:00Z }
  gamma: { description: Gamma, created_at: 2024-01-01T00:00:00Z }
  default: { description: Default, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: wildcard role }
  dev: { permissions: [view, manage, create, logs, admin_read], description: explicit list }
  viewer: { permissions: [view], description: explicit list }
  bp_only: { permissions: ['create:static-html'], description: blueprint-only role }
assignments:
  dora@other.test:
    - { role: admin, scopes: [alpha, beta] }
  carol@example.com:
    - { role: viewer, scopes: [alpha] }
  '@example.com':
    - { role: dev, scopes: [beta] }
  identifier:ci:
    - { role: bp_only, scopes: [alpha] }
    - { role: viewer, scopes: ['*'] }
  ghost@other.test:
    - { role: admin, scopes: [] }
"#;

/// Appended to [`POLICY`] for the main matrix; left out for the users without
/// any assignment.
const STAR_ASSIGNMENT: &str = r#"
  '*':
    - { role: viewer, scopes: [gamma] }
"#;

// User kinds.
/// Exact entry, asked for in a different case.
const DORA: &str = "Dora@Other.TEST";
/// Only the `@example.com` entry.
const ERIN: &str = "erin@example.com";
/// Subdomain of `@example.com`, which does not match it.
const FRANK: &str = "frank@sub.example.com";
/// Bearer-token identifier.
const CI: &str = "identifier:ci";
/// Identifier in a different case: identifiers are case-sensitive.
const CI_CASE: &str = "identifier:CI";
/// Matches only `*`.
const ZED: &str = "zed@nowhere.test";
/// Exact entry (viewer in alpha) plus the `@example.com` entry (dev in beta).
const CAROL: &str = "carol@example.com";
/// Admin role assigned with an empty scope list.
const GHOST: &str = "ghost@other.test";

const USERS: [&str; 8] = [DORA, ERIN, FRANK, CI, CI_CASE, ZED, CAROL, GHOST];

const ALL_SCOPES: [&str; 4] = ["alpha", "beta", "gamma", "default"];
const BLUEPRINTS: [&str; 2] = ["static-html", "other"];

const ALL: &[&str] = &[
    "view",
    "manage",
    "create",
    "destroy",
    "shell",
    "logs",
    "action_read",
    "action_write",
    "action_manage",
    "action_approve",
    "admin_read",
    "admin_write",
];
const DEV: &[&str] = &["view", "manage", "create", "logs", "admin_read"];
const VIEW: &[&str] = &["view"];
const NONE: &[&str] = &[];

/// Create actions: plain `create` (no blueprint) and from a blueprint.
const CREATE_ALL: &[&str] = &["create", "create:static-html", "create:other"];
const CREATE_STATIC: &[&str] = &["create:static-html"];

#[derive(Debug, Clone, Copy)]
enum Target {
    /// `check_permission(user, app, perm)`
    App(&'static str),
    /// `check_permission_in_scopes(user, scopes, perm)`
    AnyOf(&'static [&'static str]),
    /// `check_global_permission(user, perm)`
    Global,
}

#[derive(Debug, Clone, Copy)]
enum CreateTarget {
    /// `check_create_in_scopes(user, scopes, blueprint)`
    AnyOf(&'static [&'static str]),
    /// `check_create_in_every_scope(user, scopes, blueprint)`
    EveryOf(&'static [&'static str]),
}

const APP_A: Target = Target::App("app-a"); // in alpha
const APP_AB: Target = Target::App("app-ab"); // in alpha and beta
const APP_NONE: Target = Target::App("app-none"); // registered without scopes
const APP_UNKNOWN: Target = Target::App("app-unknown"); // never registered
const APP_GAMMA: Target = Target::App("gamma"); // never registered, named like a scope
const IN_ALPHA: Target = Target::AnyOf(&["alpha"]);
const IN_BETA_GAMMA: Target = Target::AnyOf(&["beta", "gamma"]);
const IN_NOTHING: Target = Target::AnyOf(&[]);
const GLOBAL: Target = Target::Global;

/// (user, target, permissions allowed). Every other permission is denied.
#[rustfmt::skip]
const PERMISSION_MATRIX: &[(&str, Target, &[&str])] = &[
    (DORA, APP_A, ALL),
    (DORA, APP_AB, ALL),
    (DORA, APP_NONE, NONE),
    (DORA, APP_UNKNOWN, NONE),
    (DORA, APP_GAMMA, NONE), // CHANGED: an unknown app named like a scope was in that scope; an app in no scope grants nothing
    (DORA, IN_ALPHA, ALL),
    (DORA, IN_BETA_GAMMA, ALL),
    (DORA, IN_NOTHING, NONE),
    (DORA, GLOBAL, ALL),

    (ERIN, APP_A, NONE),
    (ERIN, APP_AB, DEV),
    (ERIN, APP_NONE, NONE),
    (ERIN, APP_UNKNOWN, NONE),
    (ERIN, APP_GAMMA, NONE), // CHANGED: was VIEW
    (ERIN, IN_ALPHA, NONE),
    (ERIN, IN_BETA_GAMMA, DEV),
    (ERIN, IN_NOTHING, NONE),
    (ERIN, GLOBAL, DEV),

    (FRANK, APP_A, NONE),
    (FRANK, APP_AB, NONE),
    (FRANK, APP_NONE, NONE),
    (FRANK, APP_UNKNOWN, NONE),
    (FRANK, APP_GAMMA, NONE), // CHANGED: was VIEW
    (FRANK, IN_ALPHA, NONE),
    (FRANK, IN_BETA_GAMMA, VIEW),
    (FRANK, IN_NOTHING, NONE),
    (FRANK, GLOBAL, VIEW),

    (CI, APP_A, VIEW),
    (CI, APP_AB, VIEW),
    (CI, APP_NONE, NONE),
    (CI, APP_UNKNOWN, NONE),
    (CI, APP_GAMMA, NONE), // CHANGED: was VIEW
    (CI, IN_ALPHA, VIEW),
    (CI, IN_BETA_GAMMA, VIEW),
    (CI, IN_NOTHING, NONE),
    (CI, GLOBAL, VIEW),

    (CI_CASE, APP_A, NONE),
    (CI_CASE, APP_AB, NONE),
    (CI_CASE, APP_NONE, NONE),
    (CI_CASE, APP_UNKNOWN, NONE),
    (CI_CASE, APP_GAMMA, NONE), // CHANGED: was VIEW
    (CI_CASE, IN_ALPHA, NONE),
    (CI_CASE, IN_BETA_GAMMA, VIEW),
    (CI_CASE, IN_NOTHING, NONE),
    (CI_CASE, GLOBAL, VIEW),

    (ZED, APP_A, NONE),
    (ZED, APP_AB, NONE),
    (ZED, APP_NONE, NONE),
    (ZED, APP_UNKNOWN, NONE),
    (ZED, APP_GAMMA, NONE), // CHANGED: was VIEW
    (ZED, IN_ALPHA, NONE),
    (ZED, IN_BETA_GAMMA, VIEW),
    (ZED, IN_NOTHING, NONE),
    (ZED, GLOBAL, VIEW),

    (CAROL, APP_A, VIEW),
    (CAROL, APP_AB, DEV),
    (CAROL, APP_NONE, NONE),
    (CAROL, APP_UNKNOWN, NONE),
    (CAROL, APP_GAMMA, NONE), // CHANGED: was VIEW
    (CAROL, IN_ALPHA, VIEW),
    (CAROL, IN_BETA_GAMMA, DEV), // CHANGED: was VIEW, the exact entry hid the domain entry
    (CAROL, IN_NOTHING, NONE),
    (CAROL, GLOBAL, DEV), // CHANGED: was VIEW, the exact entry hid the domain entry

    (GHOST, APP_A, NONE),
    (GHOST, APP_AB, NONE),
    (GHOST, APP_NONE, NONE),
    (GHOST, APP_UNKNOWN, NONE),
    (GHOST, APP_GAMMA, NONE), // CHANGED: was VIEW
    (GHOST, IN_ALPHA, NONE),
    (GHOST, IN_BETA_GAMMA, VIEW),
    (GHOST, IN_NOTHING, NONE),
    (GHOST, GLOBAL, VIEW), // CHANGED: was ALL, the global check ignored that the admin assignment has no scopes
];

const C_ALPHA: CreateTarget = CreateTarget::AnyOf(&["alpha"]);
const C_BETA_GAMMA: CreateTarget = CreateTarget::AnyOf(&["beta", "gamma"]);
const C_EVERY_ALPHA_BETA: CreateTarget = CreateTarget::EveryOf(&["alpha", "beta"]);
const C_EVERY_NOTHING: CreateTarget = CreateTarget::EveryOf(&[]);

/// (user, target, create actions allowed) over [`CREATE_ALL`].
#[rustfmt::skip]
const CREATE_MATRIX: &[(&str, CreateTarget, &[&str])] = &[
    (DORA, C_ALPHA, CREATE_ALL),
    (DORA, C_BETA_GAMMA, CREATE_ALL),
    (DORA, C_EVERY_ALPHA_BETA, CREATE_ALL),
    (DORA, C_EVERY_NOTHING, NONE),

    (ERIN, C_ALPHA, NONE),
    (ERIN, C_BETA_GAMMA, CREATE_ALL),
    (ERIN, C_EVERY_ALPHA_BETA, NONE),
    (ERIN, C_EVERY_NOTHING, NONE),

    (FRANK, C_ALPHA, NONE),
    (FRANK, C_BETA_GAMMA, NONE),
    (FRANK, C_EVERY_ALPHA_BETA, NONE),
    (FRANK, C_EVERY_NOTHING, NONE),

    (CI, C_ALPHA, CREATE_STATIC),
    (CI, C_BETA_GAMMA, NONE),
    (CI, C_EVERY_ALPHA_BETA, NONE),
    (CI, C_EVERY_NOTHING, NONE),

    (CI_CASE, C_ALPHA, NONE),
    (CI_CASE, C_BETA_GAMMA, NONE),
    (CI_CASE, C_EVERY_ALPHA_BETA, NONE),
    (CI_CASE, C_EVERY_NOTHING, NONE),

    (ZED, C_ALPHA, NONE),
    (ZED, C_BETA_GAMMA, NONE),
    (ZED, C_EVERY_ALPHA_BETA, NONE),
    (ZED, C_EVERY_NOTHING, NONE),

    (CAROL, C_ALPHA, NONE),
    (CAROL, C_BETA_GAMMA, CREATE_ALL), // CHANGED: was NONE, the exact entry hid the domain entry
    (CAROL, C_EVERY_ALPHA_BETA, NONE),
    (CAROL, C_EVERY_NOTHING, NONE),

    (GHOST, C_ALPHA, NONE),
    (GHOST, C_BETA_GAMMA, NONE),
    (GHOST, C_EVERY_ALPHA_BETA, NONE),
    (GHOST, C_EVERY_NOTHING, NONE),
];

/// What `get_user_permissions` and `get_user_scopes_with_permissions` list,
/// per user: (scope, permission strings as in the policy).
type ScopePermissions = &'static [(&'static str, &'static [&'static str])];
#[rustfmt::skip]
const LISTINGS: &[(&str, ScopePermissions)] = &[
    (DORA, &[("alpha", &["*"]), ("beta", &["*"]), ("gamma", VIEW)]),
    (ERIN, &[("beta", DEV), ("gamma", VIEW)]),
    (FRANK, &[("gamma", VIEW)]),
    (CI, &[("alpha", &["create:static-html", "view"]), ("beta", VIEW), ("gamma", VIEW), ("default", VIEW)]),
    (CI_CASE, &[("gamma", VIEW)]),
    (ZED, &[("gamma", VIEW)]),
    (CAROL, &[("alpha", VIEW), ("beta", DEV), ("gamma", VIEW)]), // CHANGED: ("beta", DEV) was hidden by the exact entry
    (GHOST, &[("gamma", VIEW)]),
];

/// App name -> scopes, set like the app scan does.
const APPS: &[(&str, &[&str])] = &[
    ("app-a", &["alpha"]),
    ("app-ab", &["alpha", "beta"]),
    ("app-none", &[]),
];

/// An authorization service over `policy` in a fresh temp dir, with the
/// matrix apps registered.
async fn service(policy: &str) -> (Arc<AuthorizationService>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("policy.yaml"), policy).unwrap();
    let auth = AuthorizationService::new(dir.path().to_str().unwrap())
        .await
        .unwrap();
    for (app, scopes) in APPS {
        auth.set_app_scopes(app, scopes.iter().map(|s| s.to_string()).collect())
            .await
            .unwrap();
    }
    (Arc::new(auth), dir)
}

async fn matrix_service() -> (Arc<AuthorizationService>, tempfile::TempDir) {
    service(&format!("{POLICY}{STAR_ASSIGNMENT}")).await
}

fn perm(name: &str) -> Permission {
    Permission::from_str(name).unwrap()
}

fn strings(s: &[&str]) -> Vec<String> {
    s.iter().map(|s| s.to_string()).collect()
}

async fn allows(auth: &AuthorizationService, user: &str, target: Target, p: &str) -> bool {
    match target {
        Target::App(app) => auth.check_permission(user, app, &perm(p)).await,
        Target::AnyOf(scopes) => {
            auth.check_permission_in_scopes(user, &strings(scopes), &perm(p))
                .await
        }
        Target::Global => auth.check_global_permission(user, &perm(p)).await,
    }
}

/// `create` means no blueprint, `create:<bp>` a templated blueprint.
async fn allows_create(
    auth: &AuthorizationService,
    user: &str,
    target: CreateTarget,
    action: &str,
) -> bool {
    let blueprint = action.strip_prefix("create:");
    match target {
        CreateTarget::AnyOf(scopes) => {
            auth.check_create_in_scopes(user, &strings(scopes), blueprint)
                .await
        }
        CreateTarget::EveryOf(scopes) => {
            auth.check_create_in_every_scope(user, &strings(scopes), blueprint)
                .await
        }
    }
}

/// Every user has `targets` rows, each for a different target.
fn assert_complete(pairs: impl Iterator<Item = String>, targets: usize) {
    let pairs: Vec<String> = pairs.collect();
    let distinct: BTreeSet<&String> = pairs.iter().collect();
    assert_eq!(distinct.len(), pairs.len(), "duplicate (user, target) rows");
    for user in USERS {
        let rows = pairs
            .iter()
            .filter(|p| p.starts_with(&format!("{user} ")))
            .count();
        assert_eq!(rows, targets, "rows for {user}");
    }
    assert_eq!(pairs.len(), USERS.len() * targets, "rows for unknown users");
}

#[tokio::test]
async fn permission_matrix() {
    assert_complete(
        PERMISSION_MATRIX
            .iter()
            .map(|(u, t, _)| format!("{u} {t:?}")),
        9,
    );
    let (auth, _dir) = matrix_service().await;
    let mut failures = vec![];
    for (user, target, allowed) in PERMISSION_MATRIX {
        for p in ALL {
            let expected = allowed.contains(p);
            if allows(&auth, user, *target, p).await != expected {
                failures.push(format!("{user} {target:?} {p}: expected {expected}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test]
async fn create_matrix() {
    assert_complete(
        CREATE_MATRIX.iter().map(|(u, t, _)| format!("{u} {t:?}")),
        4,
    );
    let (auth, _dir) = matrix_service().await;
    let mut failures = vec![];
    for (user, target, allowed) in CREATE_MATRIX {
        for action in CREATE_ALL {
            let expected = allowed.contains(action);
            if allows_create(&auth, user, *target, action).await != expected {
                failures.push(format!("{user} {target:?} {action}: expected {expected}"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

type Listing = BTreeMap<String, BTreeSet<String>>;

fn expected_listing(user: &str) -> Listing {
    let (_, scopes) = LISTINGS.iter().find(|(u, _)| *u == user).unwrap();
    scopes
        .iter()
        .map(|(s, ps)| (s.to_string(), ps.iter().map(|p| p.to_string()).collect()))
        .collect()
}

async fn listing(auth: &AuthorizationService, user: &str) -> Listing {
    auth.get_user_permissions(user)
        .await
        .into_iter()
        .map(|(s, ps)| (s, ps.into_iter().collect()))
        .collect()
}

async fn scope_listing(auth: &AuthorizationService, user: &str) -> Listing {
    auth.get_user_scopes_with_permissions(user)
        .await
        .into_iter()
        .map(|s| (s.name, s.permissions.into_iter().collect()))
        .collect()
}

#[tokio::test]
async fn listings_match_the_table() {
    let (auth, _dir) = matrix_service().await;
    for user in USERS {
        let expected = expected_listing(user);
        assert_eq!(
            listing(&auth, user).await,
            expected,
            "permissions of {user}"
        );
        assert_eq!(
            scope_listing(&auth, user).await,
            expected,
            "scopes of {user}"
        );
    }
}

/// Whether `listing` grants `action` (a permission or `create:<bp>`) in `scope`.
fn listed(listing: &Listing, scope: &str, action: &str) -> bool {
    listing.get(scope).is_some_and(|ps| {
        ps.contains("*")
            || ps.contains(action)
            || (action.starts_with("create:") && ps.contains("create"))
    })
}

/// Compare the listing with the checks for `user`, both ways (listed implies
/// allowed, allowed implies listed). Returns the mismatches as
/// `"<check> <target> <action>: <allowed|listed> only"`.
async fn parity_mismatches(auth: &AuthorizationService, user: &str) -> Vec<String> {
    let list = listing(auth, user).await;
    let mut out = vec![];
    let mut compare = |check: &str, allowed: bool, is_listed: bool| {
        if allowed != is_listed {
            let side = if allowed { "allowed" } else { "listed" };
            out.push(format!("{check}: {side} only"));
        }
    };
    for scope in ALL_SCOPES {
        for p in ALL {
            let allowed = auth
                .check_permission_in_scopes(user, &strings(&[scope]), &perm(p))
                .await;
            compare(
                &format!("scope {scope} {p}"),
                allowed,
                listed(&list, scope, p),
            );
        }
        for bp in BLUEPRINTS {
            let action = format!("create:{bp}");
            let allowed = auth
                .check_create_in_scopes(user, &strings(&[scope]), Some(bp))
                .await;
            compare(
                &format!("scope {scope} {action}"),
                allowed,
                listed(&list, scope, &action),
            );
        }
    }
    for (app, scopes) in APPS {
        for p in ALL {
            let allowed = auth.check_permission(user, app, &perm(p)).await;
            let is_listed = scopes.iter().any(|s| listed(&list, s, p));
            compare(&format!("app {app} {p}"), allowed, is_listed);
        }
    }
    for p in ALL {
        let allowed = auth.check_global_permission(user, &perm(p)).await;
        let is_listed = ALL_SCOPES.iter().any(|s| listed(&list, s, p));
        compare(&format!("global {p}"), allowed, is_listed);
    }
    out
}

/// Disagreements between listings and checks, per user: none. (CHANGED:
/// the exact+domain user and the admin without scopes had mismatches.)
fn expected_parity_mismatches(_user: &str) -> Vec<String> {
    vec![]
}

#[tokio::test]
async fn listings_agree_with_checks() {
    let (auth, _dir) = matrix_service().await;
    for user in USERS {
        assert_eq!(
            parity_mismatches(&auth, user).await,
            expected_parity_mismatches(user),
            "listing parity of {user}"
        );
    }
}

/// Whether the OAuth session of `user` survives the session server's recheck.
async fn session_kept(auth: Arc<AuthorizationService>, user: &str) -> bool {
    let dir = tempfile::tempdir().unwrap();
    let pool = scotty::oauth::store::open(dir.path().join("s.sqlite"))
        .await
        .unwrap();
    let server = scotty::oauth::server::session_server(pool, b"characterization-secret", auth);
    server
        .issue_grant(user.to_string(), "scotty-web", "Scotty web", "api")
        .await
        .is_ok()
}

#[tokio::test]
async fn session_recheck_keeps_every_matrix_user() {
    let (auth, _dir) = matrix_service().await;
    for user in USERS {
        // Everyone holds view in gamma through `*`.
        assert!(session_kept(auth.clone(), user).await, "{user}");
    }
}

/// Without a `*` entry: a user without any assignment, and an admin whose
/// assignment has no scopes.
#[tokio::test]
async fn users_without_assignments() {
    let (auth, _dir) = service(POLICY).await;
    for user in [ZED, FRANK, CI_CASE] {
        for target in [
            APP_A,
            APP_AB,
            APP_NONE,
            APP_UNKNOWN,
            APP_GAMMA,
            IN_ALPHA,
            IN_BETA_GAMMA,
            GLOBAL,
        ] {
            for p in ALL {
                assert!(
                    !allows(&auth, user, target, p).await,
                    "{user} {target:?} {p}"
                );
            }
        }
        for target in [C_ALPHA, C_BETA_GAMMA, C_EVERY_ALPHA_BETA] {
            for action in CREATE_ALL {
                assert!(!allows_create(&auth, user, target, action).await);
            }
        }
        assert!(listing(&auth, user).await.is_empty(), "{user}");
        assert!(scope_listing(&auth, user).await.is_empty(), "{user}");
        assert_eq!(parity_mismatches(&auth, user).await, Vec::<String>::new());
        assert!(!session_kept(auth.clone(), user).await, "{user}");
    }

    // CHANGED: an admin role without scopes grants nothing, globally too
    // (the global check used to grant every permission).
    for p in ALL {
        assert!(!allows(&auth, GHOST, GLOBAL, p).await, "{p}");
        assert!(!allows(&auth, GHOST, IN_ALPHA, p).await, "{p}");
        assert!(!allows(&auth, GHOST, APP_A, p).await, "{p}");
    }
    assert!(listing(&auth, GHOST).await.is_empty());
    assert_eq!(parity_mismatches(&auth, GHOST).await, Vec::<String>::new());
    assert!(!session_kept(auth.clone(), GHOST).await);
}
