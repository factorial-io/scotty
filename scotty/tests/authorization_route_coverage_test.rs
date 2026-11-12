//! Every REST operation is listed here with the permission that guards it.
//! A user without any assignment must be refused with 403 and the standard
//! error body; a user holding exactly the listed permission must get past the
//! check. A new operation without an entry fails the test, so a route cannot be
//! added without deciding how it is protected.
//!
//! The tests run in dev auth mode, where the configured dev user is the
//! caller: unlike bearer tokens (refused with 401 at authentication when the
//! identifier has no assignment), that user passes authentication with no
//! rights at all.

use std::collections::BTreeSet;

use axum::http::Method;
use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty::app_state::SharedAppState;
use scotty::services::authorization::Permission;
use scotty_core::apps::app_data::AppData;

const APP: &str = "route-app";
const SCOPE: &str = "routes";
const NOBODY: &str = "nobody@example.com";

#[derive(Debug, Clone, Copy, PartialEq)]
enum Gate {
    /// `require_permission(<perm>)` in the router.
    Middleware(&'static str),
    /// Checked in the handler, with the same 403 refusal.
    Handler(&'static str),
    /// NOT 403: the handler filters its result by a permission instead of
    /// refusing; covered by `filtering_routes_hide_what_the_user_may_not_see`
    /// or the named suite.
    Filters(&'static str),
    /// No permission check: public, any authenticated caller, or people with
    /// a session only (covered by the OAuth tests).
    Open,
}
use Gate::*;

/// Every operation of the OpenAPI document. `[a|b]` in a handler gate means
/// either permission is enough.
#[rustfmt::skip]
const ROUTES: &[(&str, Gate)] = &[
    ("GET /api/v1/health", Open),
    ("GET /api/v1/info", Open),
    ("POST /api/v1/login", Open),
    ("POST /api/v1/authenticated/validate-token", Open),
    ("POST /api/v1/authenticated/logout", Open),
    ("GET /api/v1/authenticated/oauth/device/{user_code}", Open),
    ("POST /api/v1/authenticated/oauth/device/approve", Open),
    ("POST /api/v1/authenticated/oauth/device/deny", Open),
    ("GET /api/v1/authenticated/oauth/consent/{id}", Open),
    ("POST /api/v1/authenticated/oauth/consent/{id}/approve", Open),
    ("POST /api/v1/authenticated/oauth/consent/{id}/deny", Open),
    // `?all=true` and revoking someone else's connection need global
    // admin_write; people only, covered by the OAuth connection tests.
    ("GET /api/v1/authenticated/connections", Open),
    ("DELETE /api/v1/authenticated/connections/{id}", Open),

    ("GET /api/v1/authenticated/apps/list", Filters("view")),
    ("GET /api/v1/authenticated/blueprints", Filters("create")),
    ("GET /api/v1/authenticated/scopes/list", Filters("any assignment")),
    ("GET /api/v1/authenticated/tasks", Filters("task visibility (test_task_visibility)")),
    ("GET /api/v1/authenticated/task/{uuid}", Filters("task visibility (test_task_visibility), 404")),

    ("GET /api/v1/authenticated/apps/info/{app_id}", Middleware("view")),
    ("GET /api/v1/authenticated/apps/run/{app_id}", Middleware("manage")),
    ("GET /api/v1/authenticated/apps/stop/{app_id}", Middleware("manage")),
    ("GET /api/v1/authenticated/apps/purge/{app_id}", Middleware("manage")),
    ("GET /api/v1/authenticated/apps/rebuild/{app_id}", Middleware("manage")),
    ("GET /api/v1/authenticated/apps/destroy/{app_id}", Middleware("destroy")),
    ("GET /api/v1/authenticated/apps/adopt/{app_id}", Middleware("create")),
    ("GET /api/v1/apps/{app_id}/services/{service}/files", Middleware("shell")),
    ("PUT /api/v1/apps/{app_id}/services/{service}/files", Middleware("manage")),
    ("GET /api/v1/authenticated/apps/{app_name}/custom-actions", Middleware("action_manage")),
    ("POST /api/v1/authenticated/apps/{app_name}/custom-actions", Middleware("action_manage")),
    ("GET /api/v1/authenticated/apps/{app_name}/custom-actions/{action_name}", Middleware("action_manage")),
    ("DELETE /api/v1/authenticated/apps/{app_name}/custom-actions/{action_name}", Middleware("action_manage")),
    ("GET /api/v1/authenticated/admin/actions/pending", Middleware("action_approve")),
    ("GET /api/v1/authenticated/admin/actions/{app_name}/{action_name}", Middleware("action_approve")),
    ("POST /api/v1/authenticated/admin/actions/{app_name}/{action_name}/approve", Middleware("action_approve")),
    ("POST /api/v1/authenticated/admin/actions/{app_name}/{action_name}/reject", Middleware("action_approve")),
    ("POST /api/v1/authenticated/admin/actions/{app_name}/{action_name}/revoke", Middleware("action_approve")),
    ("GET /api/v1/authenticated/admin/scopes", Middleware("admin_read")),
    ("POST /api/v1/authenticated/admin/scopes", Middleware("admin_write")),
    ("GET /api/v1/authenticated/admin/roles", Middleware("admin_read")),
    ("POST /api/v1/authenticated/admin/roles", Middleware("admin_write")),
    ("GET /api/v1/authenticated/admin/assignments", Middleware("admin_read")),
    ("POST /api/v1/authenticated/admin/assignments", Middleware("admin_write")),
    ("DELETE /api/v1/authenticated/admin/assignments", Middleware("admin_write")),
    ("GET /api/v1/authenticated/admin/permissions", Middleware("admin_read")),
    ("POST /api/v1/authenticated/admin/permissions/test", Middleware("admin_read")),
    ("GET /api/v1/authenticated/admin/permissions/user/{user_id}", Middleware("admin_read")),

    ("POST /api/v1/authenticated/apps/create", Handler("create")),
    ("POST /api/v1/authenticated/apps/notify/add", Handler("manage")),
    ("POST /api/v1/authenticated/apps/notify/remove", Handler("manage")),
    ("POST /api/v1/authenticated/apps/{app_name}/actions", Handler("action_read|action_write")),
];

/// `require_permission(` calls in router.rs. Three of them guard two methods
/// each (custom-actions, custom-actions/{action_name}, admin/assignments),
/// hence 28 middleware operations.
const REQUIRE_PERMISSION_CALLS: usize = 25;
const MIDDLEWARE_OPERATIONS: usize = 28;

/// A policy with one role per permission, each held in [`SCOPE`] by
/// `holder-<permission>@example.com`, and no `*` entry.
fn policy() -> String {
    let mut roles = String::new();
    let mut assignments = String::new();
    for p in Permission::all() {
        let p = p.as_str();
        roles.push_str(&format!(
            "  only_{p}: {{ permissions: [{p}], description: d }}\n"
        ));
        assignments.push_str(&format!(
            "  holder-{p}@example.com:\n    - {{ role: only_{p}, scopes: [{SCOPE}] }}\n"
        ));
    }
    format!(
        "scopes:\n  {SCOPE}: {{ description: d, created_at: 2024-01-01T00:00:00Z }}\n\
         roles:\n{roles}assignments:\n{assignments}"
    )
}

/// App state in dev auth mode with `user` as the caller, over a fresh copy of
/// [`policy`], with [`APP`] assigned to [`SCOPE`].
async fn state_for(user: &str) -> (SharedAppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("policy.yaml"), policy()).unwrap();

    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.auth_mode", "dev")
        .unwrap()
        .set_override("api.dev_user_email", user)
        .unwrap()
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings, None, dir.path().to_str().unwrap()).await;
    state
        .auth_service
        .set_app_scopes(APP, vec![SCOPE.to_string()])
        .await
        .unwrap();
    (state, dir)
}

fn holder(permission: &str) -> String {
    format!("holder-{permission}@example.com")
}

fn concrete_path(template: &str) -> String {
    template
        .replace("{app_id}", APP)
        .replace("{app_name}", APP)
        .replace("{action_name}", "route-action")
        .replace("{service}", "web")
        .replace("{user_id}", "someone@example.com")
        .replace("{uuid}", "00000000-0000-0000-0000-000000000000")
        .replace("{id}", "00000000-0000-0000-0000-000000000000")
        .replace("{user_code}", "BBBB-CCCC")
}

/// Bodies for handlers that check after parsing the body; valid enough to
/// reach the check, and rejected right after it without side effects.
fn body_for(op: &str) -> Option<serde_json::Value> {
    let (method, path) = op.split_once(' ').unwrap();
    match path {
        "/api/v1/authenticated/apps/create" => Some(serde_json::json!({
            "app_name": APP,
            "settings": {
                "public_services": [],
                "domain": "",
                "time_to_live": "Forever",
                "basic_auth": null,
                "disallow_robots": true,
                "environment": {},
                "registry": null,
                "app_blueprint": null,
                "notify": [],
                "middlewares": [],
                "scopes": []
            },
            // A .scotty.yml file is rejected right after the permission check.
            "files": { "files": [{ "name": ".scotty.yml", "content": "" }] },
            "custom_domains": [],
            "requested_scopes": [SCOPE]
        })),
        "/api/v1/authenticated/apps/notify/add" | "/api/v1/authenticated/apps/notify/remove" => {
            Some(serde_json::json!({ "app_name": APP, "service_ids": [] }))
        }
        "/api/v1/authenticated/apps/{app_name}/actions" => {
            Some(serde_json::json!({ "action_name": "route-action" }))
        }
        // Route-level checks run before the body is parsed; an empty object
        // makes allowed requests fail validation instead of changing state.
        _ if method != "GET" => Some(serde_json::json!({})),
        _ => None,
    }
}

async fn call(server: &TestServer, op: &str) -> axum_test::TestResponse {
    let (method, path) = op.split_once(' ').unwrap();
    let mut request = server.method(
        Method::from_bytes(method.as_bytes()).unwrap(),
        &concrete_path(path),
    );
    if let Some(body) = body_for(op) {
        request = request.json(&body);
    }
    request.await
}

fn gated() -> impl Iterator<Item = (&'static str, &'static str)> {
    ROUTES.iter().filter_map(|(op, gate)| match gate {
        Middleware(p) | Handler(p) => Some((*op, *p)),
        _ => None,
    })
}

#[tokio::test]
async fn every_operation_has_a_gate() {
    let (state, _dir) = state_for(NOBODY).await;
    let server = TestServer::new(ApiRoutes::create(state));
    let doc: serde_json::Value = server.get("/api-docs/openapi.json").await.json();
    let mut documented = BTreeSet::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            documented.insert(format!("{} {path}", method.to_uppercase()));
        }
    }
    let listed: BTreeSet<String> = ROUTES.iter().map(|(op, _)| op.to_string()).collect();
    assert_eq!(listed.len(), ROUTES.len(), "duplicate entries");
    assert_eq!(
        documented.difference(&listed).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "operations without a gate"
    );
    assert_eq!(
        listed.difference(&documented).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "gates for operations that no longer exist"
    );

    let source = std::fs::read_to_string("src/api/router.rs").unwrap();
    assert_eq!(
        source.matches("require_permission(").count(),
        REQUIRE_PERMISSION_CALLS,
        "require_permission uses in router.rs changed; update ROUTES"
    );
    let middleware = ROUTES
        .iter()
        .filter(|(_, g)| matches!(g, Middleware(_)))
        .count();
    assert_eq!(middleware, MIDDLEWARE_OPERATIONS);
}

#[tokio::test]
async fn user_without_assignment_is_refused_with_403_and_reason() {
    let (state, _dir) = state_for(NOBODY).await;
    let server = TestServer::new(ApiRoutes::create(state));
    let mut failures = vec![];
    for (op, _) in gated() {
        let response = call(&server, op).await;
        let status = response.status_code().as_u16();
        let body: serde_json::Value =
            serde_json::from_str(&response.text()).unwrap_or(serde_json::Value::Null);
        let standard_body =
            body["error"] == true && body["message"].as_str().is_some_and(|m| !m.is_empty());
        if status != 403 || !standard_body {
            failures.push(format!("{op}: {status} {body}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test]
async fn holder_of_exactly_the_permission_gets_past_the_check() {
    let mut servers = std::collections::BTreeMap::new();
    let mut failures = vec![];
    for (op, permissions) in gated() {
        for permission in permissions.split('|') {
            if !servers.contains_key(permission) {
                let (state, dir) = state_for(&holder(permission)).await;
                let server = TestServer::new(ApiRoutes::create(state));
                servers.insert(permission, (server, dir));
            }
            let response = call(&servers[permission].0, op).await;
            let status = response.status_code().as_u16();
            // 5xx would hide a check that fails for everyone. CHANGED: custom
            // action create/approve/reject/revoke answered 500 to everyone.
            if status == 401 || status == 403 || status >= 500 {
                failures.push(format!(
                    "{op} as holder of {permission}: {status} {}",
                    response.text()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// The listing routes answer 200 to everyone and leave out what the caller may
/// not see.
#[tokio::test]
async fn filtering_routes_hide_what_the_user_may_not_see() {
    let names = |v: &serde_json::Value, key: &str| -> Vec<String> {
        match &v[key] {
            serde_json::Value::Array(items) => items
                .iter()
                .map(|i| i["name"].as_str().unwrap().to_string())
                .collect(),
            serde_json::Value::Object(map) => map.keys().cloned().collect(),
            other => panic!("unexpected {key}: {other}"),
        }
    };
    let get = |user: String, path: &'static str| async move {
        let (state, _dir) = state_for(&user).await;
        // Only listed, never inspected: the record has no files behind it.
        state
            .apps
            .add_app(AppData {
                name: APP.to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        let response = TestServer::new(ApiRoutes::create(state)).get(path).await;
        assert_eq!(response.status_code(), 200, "{path} as {user}");
        response.json::<serde_json::Value>()
    };

    let apps = "/api/v1/authenticated/apps/list";
    assert!(names(&get(NOBODY.into(), apps).await, "apps").is_empty());
    assert_eq!(names(&get(holder("view"), apps).await, "apps"), [APP]);
    assert!(names(&get(holder("manage"), apps).await, "apps").is_empty());

    let blueprints = "/api/v1/authenticated/blueprints";
    assert!(names(&get(NOBODY.into(), blueprints).await, "blueprints").is_empty());
    assert_eq!(
        names(&get(holder("create"), blueprints).await, "blueprints"),
        ["test-blueprint"]
    );

    let scopes = "/api/v1/authenticated/scopes/list";
    assert!(names(&get(NOBODY.into(), scopes).await, "scopes").is_empty());
    assert_eq!(names(&get(holder("logs"), scopes).await, "scopes"), [SCOPE]);
}
