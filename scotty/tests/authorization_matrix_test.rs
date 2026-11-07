//! Authorization matrix: every REST operation and every app-scoped WebSocket
//! message is called by every test identity, and the outcome (allowed or
//! denied) is compared with the expectation in
//! `tests/fixtures/authorization_matrix.yaml`.
//!
//! Expectations name a permission, not per-role outcomes. Which identity holds
//! which permission comes from the fixture policy in
//! `tests/fixtures/casbin-matrix/policy.yaml`, so a role change there is picked
//! up automatically.
//!
//! "Allowed" means the request got past authorization: any status other than
//! 401/403 for REST, or any WebSocket reply other than an authentication or
//! access-denied error. The fixture app is registered in a scope but has no
//! files, so allowed requests usually end in "not found" without side effects.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;

use axum::extract::ws::Message;
use axum::http::Method;
use axum_test::TestServer;
use scotty::api::auth_core::CurrentUser;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty::api::websocket::handlers::handle_websocket_message;
use scotty::app_state::{SharedAppState, WebSocketClient};
use scotty_core::websocket::message::WebSocketMessage;
use serde::Deserialize;
use uuid::Uuid;

const MATRIX_FIXTURE: &str = "tests/fixtures/authorization_matrix.yaml";
const POLICY_FIXTURE_DIR: &str = "tests/fixtures/casbin-matrix";
const APP: &str = "matrix-app";
const APP_SCOPE: &str = "matrix";
/// Identity with a configured token but no role assignment. Such tokens are
/// rejected at authentication (401), so `nobody` may only use public routes.
const NOBODY: &str = "nobody";

#[derive(Deserialize)]
struct Matrix {
    rest: BTreeMap<String, Expectation>,
    websocket: BTreeMap<String, Expectation>,
    /// Operations that take the app (or its scopes) from the request body.
    app_scoped_by_body: Vec<String>,
    undocumented_routes: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
enum Expectation {
    One(String),
    AnyOf(Vec<String>),
}

#[derive(Deserialize)]
struct Policy {
    roles: HashMap<String, Role>,
    assignments: HashMap<String, Vec<Assignment>>,
}

#[derive(Deserialize)]
struct Role {
    permissions: Vec<String>,
}

#[derive(Deserialize)]
struct Assignment {
    role: String,
    scopes: Vec<String>,
}

fn load_matrix() -> Matrix {
    serde_norway::from_str(&std::fs::read_to_string(MATRIX_FIXTURE).unwrap()).unwrap()
}

fn load_policy() -> Policy {
    let path = Path::new(POLICY_FIXTURE_DIR).join("policy.yaml");
    serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Test identities: every assigned identifier plus one without assignments.
fn identities(policy: &Policy) -> Vec<String> {
    let mut ids: Vec<String> = policy
        .assignments
        .keys()
        .filter_map(|k| k.strip_prefix("identifier:").map(str::to_string))
        .collect();
    ids.push(NOBODY.to_string());
    ids.sort();
    ids
}

fn token_for(identity: &str) -> String {
    format!("matrix-token-{identity}")
}

/// Does `identity` hold `permission`, in `scope` or (if `None`) in any scope?
fn holds(policy: &Policy, identity: &str, permission: &str, scope: Option<&str>) -> bool {
    let Some(assignments) = policy.assignments.get(&format!("identifier:{identity}")) else {
        return false;
    };
    assignments.iter().any(|a| {
        let role = &policy.roles[&a.role];
        let has_permission = role.permissions.iter().any(|p| p == "*" || p == permission);
        let in_scope = match scope {
            None => true,
            Some(scope) => a.scopes.iter().any(|s| s == "*" || s == scope),
        };
        has_permission && in_scope
    })
}

/// Whether `identity` should get past authorization for an operation.
///
/// `app_scoped` is true when the operation names the fixture app, in its path
/// or in its body; app permissions are then checked in the app's scope instead
/// of in any scope.
fn expected_allowed(policy: &Policy, identity: &str, app_scoped: bool, exp: &Expectation) -> bool {
    let permissions = match exp {
        Expectation::One(p) => vec![p.clone()],
        Expectation::AnyOf(ps) => ps.clone(),
    };
    permissions
        .iter()
        .any(|permission| match permission.as_str() {
            "public" => true,
            _ if identity == NOBODY => false,
            "authenticated" => true,
            p if p.starts_with("admin_") => holds(policy, identity, p, None),
            p => holds(policy, identity, p, app_scoped.then_some(APP_SCOPE)),
        })
}

fn names_app(matrix: &Matrix, op: &str) -> bool {
    let path = split_op(op).1;
    path.contains("{app_id}")
        || path.contains("{app_name}")
        || matrix.app_scoped_by_body.iter().any(|o| o == op)
}

fn is_public(exp: &Expectation) -> bool {
    matches!(exp, Expectation::One(p) if p == "public")
}

/// Build app state with the matrix policy copied to a temporary directory and
/// one bearer token per identity.
async fn matrix_state(policy: &Policy) -> (SharedAppState, tempfile::TempDir) {
    let policy_dir = tempfile::tempdir().unwrap();
    for file in ["model.conf", "policy.yaml"] {
        std::fs::copy(
            Path::new(POLICY_FIXTURE_DIR).join(file),
            policy_dir.path().join(file),
        )
        .unwrap();
    }

    // ApiServer fields are read-only, so tokens and limits go in as config
    // overrides. Keep rate limits from turning the many requests into 429s.
    let mut builder = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap();
    for id in identities(policy) {
        builder = builder
            .set_override(format!("api.bearer_tokens.{id}"), token_for(&id))
            .unwrap();
    }
    let settings: scotty::settings::config::Settings =
        builder.build().unwrap().try_deserialize().unwrap();

    let state =
        create_test_app_state_with_policy_dir(settings, None, policy_dir.path().to_str().unwrap())
            .await;
    state
        .auth_service
        .set_app_scopes(APP, vec![APP_SCOPE.to_string()])
        .await
        .unwrap();
    (state, policy_dir)
}

/// Fill path parameters with fixture values.
fn concrete_path(template: &str) -> String {
    template
        .replace("{app_id}", APP)
        .replace("{app_name}", APP)
        .replace("{action_name}", "matrix-action")
        .replace("{service}", "web")
        .replace("{uuid}", &Uuid::new_v4().to_string())
        .replace("{user_id}", "someone@example.com")
}

/// Request bodies for handlers that check permissions themselves, after the
/// body has been parsed. Each is valid enough to reach the check and is then
/// rejected without side effects.
fn body_for(method: &str, path: &str) -> Option<serde_json::Value> {
    match (method, path) {
        ("POST", "/api/v1/authenticated/apps/create") => Some(serde_json::json!({
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
            "requested_scopes": [APP_SCOPE]
        })),
        ("POST", "/api/v1/authenticated/apps/notify/add")
        | ("POST", "/api/v1/authenticated/apps/notify/remove") => Some(serde_json::json!({
            "app_name": APP,
            "service_ids": []
        })),
        ("POST", "/api/v1/authenticated/apps/{app_name}/actions") => {
            Some(serde_json::json!({ "action_name": "matrix-action" }))
        }
        // Route-level checks run before the body is parsed; an empty object
        // makes allowed requests fail validation instead of changing state.
        ("POST", _) | ("PUT", _) | ("DELETE", _) => Some(serde_json::json!({})),
        _ => None,
    }
}

async fn openapi_operations(server: &TestServer) -> BTreeSet<String> {
    let doc: serde_json::Value = server.get("/api-docs/openapi.json").await.json();
    let mut ops = BTreeSet::new();
    for (path, item) in doc["paths"].as_object().unwrap() {
        for method in item.as_object().unwrap().keys() {
            ops.insert(format!("{} {}", method.to_uppercase(), path));
        }
    }
    ops
}

fn split_op(op: &str) -> (&str, &str) {
    op.split_once(' ').unwrap()
}

#[tokio::test]
async fn every_openapi_operation_has_an_expectation() {
    let matrix = load_matrix();
    let policy = load_policy();
    let (state, _dir) = matrix_state(&policy).await;
    let server = TestServer::new(ApiRoutes::create(state));

    let documented = openapi_operations(&server).await;
    let expected: BTreeSet<String> = matrix.rest.keys().cloned().collect();

    let unmapped: Vec<_> = documented.difference(&expected).collect();
    let stale: Vec<_> = expected.difference(&documented).collect();
    assert!(
        unmapped.is_empty(),
        "operations without an entry in {MATRIX_FIXTURE}: {unmapped:#?}"
    );
    assert!(
        stale.is_empty(),
        "entries in {MATRIX_FIXTURE} for operations that no longer exist: {stale:#?}"
    );
}

#[tokio::test]
async fn every_router_route_is_documented() {
    let matrix = load_matrix();
    let policy = load_policy();
    let (state, _dir) = matrix_state(&policy).await;
    let server = TestServer::new(ApiRoutes::create(state));

    let documented: BTreeSet<String> = openapi_operations(&server)
        .await
        .iter()
        .map(|op| split_op(op).1.to_string())
        .collect();

    let source = std::fs::read_to_string("src/api/router.rs").unwrap();
    let route_re = regex::Regex::new(r#"\.route\(\s*"([^"]+)""#).unwrap();
    let registered: BTreeSet<String> = route_re
        .captures_iter(&source)
        .map(|c| c[1].to_string())
        .collect();
    assert!(!registered.is_empty(), "no routes found in router.rs");

    let allowed: BTreeSet<String> = matrix.undocumented_routes.iter().cloned().collect();
    let undocumented: Vec<_> = registered
        .iter()
        .filter(|r| !documented.contains(*r) && !allowed.contains(*r))
        .collect();
    assert!(
        undocumented.is_empty(),
        "routes in router.rs missing from the OpenAPI document (add a utoipa \
         path or list them under undocumented_routes in {MATRIX_FIXTURE}): {undocumented:#?}"
    );
}

#[tokio::test]
async fn rest_operations_require_a_valid_token() {
    let matrix = load_matrix();
    let policy = load_policy();
    let (state, _dir) = matrix_state(&policy).await;
    let server = TestServer::new(ApiRoutes::create(state));

    let mut failures = vec![];
    for (op, exp) in &matrix.rest {
        if is_public(exp) {
            continue;
        }
        let (method, path) = split_op(op);
        for token in [None, Some("not-a-valid-token")] {
            let mut request = server.method(
                Method::from_bytes(method.as_bytes()).unwrap(),
                &concrete_path(path),
            );
            if let Some(body) = body_for(method, path) {
                request = request.json(&body);
            }
            if let Some(t) = token {
                request = request.authorization_bearer(t);
            }
            let status = request.await.status_code().as_u16();
            if status != 401 {
                failures.push(format!(
                    "{op} with token {token:?}: expected 401, got {status}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[tokio::test]
async fn rest_operations_match_the_matrix() {
    let matrix = load_matrix();
    let policy = load_policy();
    let (state, _dir) = matrix_state(&policy).await;
    let server = TestServer::new(ApiRoutes::create(state));

    let mut failures = vec![];
    for (op, exp) in &matrix.rest {
        let (method, path) = split_op(op);
        for identity in identities(&policy) {
            let mut request = server
                .method(
                    Method::from_bytes(method.as_bytes()).unwrap(),
                    &concrete_path(path),
                )
                .authorization_bearer(token_for(&identity));
            if let Some(body) = body_for(method, path) {
                request = request.json(&body);
            }
            let response = request.await;
            let status = response.status_code().as_u16();
            let allowed = status != 401 && status != 403;
            let expected = expected_allowed(&policy, &identity, names_app(&matrix, op), exp);
            if allowed != expected {
                failures.push(format!(
                    "{op} as {identity}: expected {}, got {status} ({})",
                    if expected { "allowed" } else { "denied" },
                    response.text().chars().take(120).collect::<String>()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches: {failures:#?}",
        failures.len()
    );
}

/// Send `msg` as `identity` and report whether it got past authorization.
async fn websocket_allowed(state: &SharedAppState, identity: &str, msg: &WebSocketMessage) -> bool {
    let client_id = Uuid::new_v4();
    let (tx, mut rx) = tokio::sync::broadcast::channel(64);
    state
        .messenger
        .add_client(client_id, WebSocketClient::new(tx))
        .await;
    state
        .messenger
        .authenticate_client(
            client_id,
            CurrentUser {
                email: format!("identifier:{identity}"),
                name: identity.to_string(),
                picture: None,
                access_token: None,
            },
        )
        .await
        .unwrap();

    handle_websocket_message(state, client_id, msg).await;

    let mut allowed = true;
    while let Ok(Ok(Message::Text(text))) =
        tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
    {
        if let Ok(WebSocketMessage::Error(err)) = serde_json::from_str(&text) {
            if err.contains("Access denied") || err.contains("Authentication required") {
                allowed = false;
            }
        }
    }
    state.messenger.remove_client(client_id).await;
    allowed
}

fn websocket_message(name: &str) -> WebSocketMessage {
    match name {
        "StartLogStream" => WebSocketMessage::StartLogStream(scotty_types::LogStreamRequest::new(
            APP.to_string(),
            "web".to_string(),
            false,
        )),
        "CreateShellSession" => WebSocketMessage::CreateShellSession(
            scotty_types::ShellSessionRequest::new(APP.to_string(), "web".to_string()),
        ),
        other => panic!("no fixture message for WebSocket message type {other}"),
    }
}

/// Allowed WebSocket requests reach code that records metrics; outside of
/// `cfg(test)` the recorder must be initialised first.
fn init_metrics_once() {
    #[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
    {
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            scotty::metrics::init_metrics().expect("failed to initialise metrics");
        });
    }
}

#[tokio::test]
async fn websocket_messages_match_the_matrix() {
    init_metrics_once();
    let matrix = load_matrix();
    let policy = load_policy();
    let (state, _dir) = matrix_state(&policy).await;

    let mut failures = vec![];
    for (name, exp) in &matrix.websocket {
        let msg = websocket_message(name);
        for identity in identities(&policy) {
            let allowed = websocket_allowed(&state, &identity, &msg).await;
            // WebSocket messages always name the fixture app.
            let expected = expected_allowed(&policy, &identity, true, exp);
            if allowed != expected {
                failures.push(format!(
                    "{name} as {identity}: expected {}, got {}",
                    if expected { "allowed" } else { "denied" },
                    if allowed { "allowed" } else { "denied" },
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches: {failures:#?}",
        failures.len()
    );
}
