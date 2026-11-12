//! The MCP endpoint: transport, authentication, tool scopes and tools.

use std::sync::Arc;

use axum::http::header;
use axum_test::{TestResponse, TestServer};
use base64::prelude::*;
use scotty_core::apps::app_data::{AppData, AppSettings, ContainerState};
use scotty_core::utils::secret::SecretHashMap;
use serde_json::{json, Value};

use super::mcp_oauth_tests::{app, assert_challenge, connect, session, settings, ADMIN};
use crate::api::router::ApiRoutes;
use crate::api::test_utils::{
    create_test_app_state_with_policy_dir, create_test_app_state_with_settings,
};
use crate::app_state::AppState;

/// Configured bearer token of `identifier:admin` (admin in every scope).
const ADMIN_TOKEN: &str = "test-bearer-token-123";
/// Configured bearer token of `identifier:restricted`, who may only view
/// and create from `static-html`, in scope `default`.
const RESTRICTED_TOKEN: &str = "restricted-token-789";

/// oauth mode with the shipped blueprints, apps under a temp root, the
/// `restricted` bearer token and `admin@example.com` an admin in `default`.
async fn scotty() -> (TestServer, Arc<AppState>) {
    scotty_with(&[]).await
}

async fn scotty_with(overrides: &[(&str, &str)]) -> (TestServer, Arc<AppState>) {
    let root = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
    let mut overrides = overrides.to_vec();
    overrides.extend([
        ("api.bearer_tokens.restricted", RESTRICTED_TOKEN),
        ("apps.root_folder", root.to_str().unwrap()),
    ]);
    let mut settings = settings(&overrides);
    for dir in ["../config/blueprints", "tests/fixtures/blueprints"] {
        scotty_core::settings::app_blueprint::load_blueprint_dirs(
            std::path::Path::new(dir),
            &mut settings.apps.blueprints,
        )
        .unwrap();
    }

    let dir = tempfile::tempdir().unwrap().keep();
    let policy = std::fs::read_to_string("tests/fixtures/policy/policy.yaml")
        .unwrap()
        .replace(
            "roles:\n",
            "roles:\n  artifacts:\n    permissions: [view, 'create:static-html']\n    description: Artifacts only\n",
        )
        .replace(
            "assignments:\n",
            &format!(
                "assignments:\n  {ADMIN}:\n    - role: admin\n      scopes: [default]\n  \
                 identifier:restricted:\n    - role: artifacts\n      scopes: [default]\n"
            ),
        );
    std::fs::write(dir.join("policy.yaml"), policy).unwrap();
    let state = create_test_app_state_with_policy_dir(settings, None, dir.to_str().unwrap()).await;
    (TestServer::new(ApiRoutes::create(state.clone())), state)
}

/// `tools/call`; the result (with `isError` set for tool errors).
async fn call(server: &TestServer, token: &str, tool: &str, arguments: Value) -> Value {
    rpc(
        server,
        token,
        "tools/call",
        json!({"name": tool, "arguments": arguments}),
    )
    .await
}

fn is_error(result: &Value) -> bool {
    result["isError"] == true
}

/// The text of a tool result's first content block.
fn text(result: &Value) -> String {
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// A running atelier app in `default` whose settings hold secrets.
async fn add_atelier_app(state: &AppState) {
    let settings = AppSettings {
        app_blueprint: Some("atelier".into()),
        scopes: vec!["default".into()],
        environment: SecretHashMap::from_hashmap(
            [
                ("HASH_SALT".to_string(), "salt-0123456789abcdef".to_string()),
                (
                    "SITE_NAME".to_string(),
                    "visible-only-to-compose".to_string(),
                ),
            ]
            .into(),
        ),
        generated_parameters: vec!["HASH_SALT".into()],
        basic_auth: Some(("user".into(), "basic-auth-pw".into())),
        ..Default::default()
    };
    state
        .apps
        .add_app(AppData {
            name: "atelier-demo".into(),
            settings: Some(settings),
            services: vec![ContainerState {
                service: "edge".into(),
                domains: vec!["edge.atelier-demo.test.local".into()],
                basic_auth: Some(("user".into(), "basic-auth-pw".into())),
                ..Default::default()
            }],
            ..Default::default()
        })
        .await
        .unwrap();
    state
        .auth_service
        .set_app_scopes("atelier-demo", vec!["default".into()])
        .await
        .unwrap();
}

async fn post(server: &TestServer, token: Option<&str>, body: Value) -> TestResponse {
    let mut request = server
        .post("/mcp")
        .add_header(header::ACCEPT, "application/json, text/event-stream")
        .json(&body);
    if let Some(token) = token {
        request = request.authorization_bearer(token);
    }
    request.await
}

/// One JSON-RPC call; the `result` (or the whole body for an error).
async fn rpc(server: &TestServer, token: &str, method: &str, params: Value) -> Value {
    let response = post(
        server,
        Some(token),
        json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
    )
    .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let body: Value = response.json();
    body.get("result").cloned().unwrap_or(body)
}

fn tool_names(list: &Value) -> Vec<String> {
    let mut names: Vec<String> = list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

async fn bearer_mode_server() -> TestServer {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    TestServer::new(ApiRoutes::create(
        create_test_app_state_with_settings(settings, None).await,
    ))
}

// --- 2.1 Endpoint and auth ------------------------------------------------------

#[tokio::test]
async fn mcp_and_bearer_tokens_reach_tools_list() {
    let (server, state) = app().await;
    let (_client, tokens) = connect(&server, &state, "alice@example.com", "read", &["read"]).await;
    let mcp_token = tokens["access_token"].as_str().unwrap();

    let init = rpc(
        &server,
        mcp_token,
        "initialize",
        json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"},
        }),
    )
    .await;
    assert_eq!(init["serverInfo"]["name"], "scotty", "{init}");
    assert!(init["capabilities"]["tools"].is_object(), "{init}");

    let by_mcp = rpc(&server, mcp_token, "tools/list", json!({})).await;
    let by_bearer = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    assert!(by_mcp["tools"].is_array(), "{by_mcp}");
    assert_eq!(tool_names(&by_mcp), tool_names(&by_bearer));
}

#[tokio::test]
async fn bearer_mode_serves_mcp_to_configured_tokens() {
    let server = bearer_mode_server().await;
    let listed = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    assert!(listed["tools"].is_array(), "{listed}");
    assert_eq!(post(&server, None, json!({})).await.status_code(), 401);
}

#[tokio::test]
async fn missing_invalid_and_session_tokens_get_the_challenge() {
    let (server, state) = app().await;
    let list = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"});
    assert_challenge(&post(&server, None, list.clone()).await);
    assert_challenge(&post(&server, Some("garbage"), list.clone()).await);
    let alice_session = session(&state, "alice@example.com").await;
    assert_challenge(&post(&server, Some(&alice_session), list).await);
}

// --- 3.2 list_blueprints, list_apps, get_app ------------------------------------

#[tokio::test]
async fn restricted_user_sees_only_static_html() {
    let (server, _state) = scotty().await;
    let names = |result: &Value| -> Vec<String> {
        result["structuredContent"]["blueprints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["name"].as_str().unwrap().to_string())
            .collect()
    };

    let restricted = call(&server, RESTRICTED_TOKEN, "list_blueprints", json!({})).await;
    assert_eq!(names(&restricted), ["static-html"], "{restricted}");
    let bp = &restricted["structuredContent"]["blueprints"][0];
    assert_eq!(bp["payload"], json!(["html/**"]));
    assert!(bp["images"].as_array().is_some_and(|i| !i.is_empty()));

    // Validation-only blueprints are not offered: create_app can't use them.
    let admin = call(&server, ADMIN_TOKEN, "list_blueprints", json!({})).await;
    assert_eq!(
        names(&admin),
        ["atelier", "demo-template", "static-html"],
        "{admin}"
    );
}

#[tokio::test]
async fn get_and_list_apps_show_no_environment_values() {
    let (server, state) = scotty().await;
    add_atelier_app(&state).await;

    let app = call(
        &server,
        ADMIN_TOKEN,
        "get_app",
        json!({"app_name": "atelier-demo"}),
    )
    .await;
    assert!(!is_error(&app), "{app}");
    assert_eq!(app["structuredContent"]["blueprint"], "atelier");
    assert_eq!(
        app["structuredContent"]["urls"],
        json!(["http://edge.atelier-demo.test.local"])
    );
    let listed = call(&server, ADMIN_TOKEN, "list_apps", json!({})).await;
    assert_eq!(
        listed["structuredContent"]["apps"][0]["name"],
        "atelier-demo"
    );
    for result in [&app, &listed] {
        let raw = result.to_string();
        for secret in [
            "salt-0123456789abcdef",
            "HASH_SALT",
            "visible-only-to-compose",
            "basic-auth-pw",
        ] {
            assert!(!raw.contains(secret), "{secret} leaked: {raw}");
        }
    }
}

#[tokio::test]
async fn apps_outside_the_callers_scopes_are_hidden() {
    let (server, state) = scotty().await;
    add_atelier_app(&state).await;
    state
        .auth_service
        .set_app_scopes("atelier-demo", vec!["client-b".into()])
        .await
        .unwrap();

    let listed = call(&server, RESTRICTED_TOKEN, "list_apps", json!({})).await;
    assert_eq!(listed["structuredContent"]["apps"], json!([]), "{listed}");
    let app = call(
        &server,
        RESTRICTED_TOKEN,
        "get_app",
        json!({"app_name": "atelier-demo"}),
    )
    .await;
    assert!(is_error(&app), "{app}");
    assert!(text(&app).contains("Access denied"), "{app}");
}

#[tokio::test]
async fn read_tools_are_annotated_read_only() {
    let (server, _state) = scotty().await;
    let list = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    for name in ["list_blueprints", "list_apps", "get_app"] {
        let tool = list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap_or_else(|| panic!("{name} missing: {list}"));
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        assert!(tool["inputSchema"].is_object(), "{tool}");
    }
}

// --- 2.2 Per-tool scopes ---------------------------------------------------------

#[tokio::test]
async fn read_tools_need_the_read_scope() {
    let (server, state) = scotty().await;
    let (_client, tokens) = connect(&server, &state, ADMIN, "create", &["create"]).await;
    let create_only = tokens["access_token"].as_str().unwrap();
    for tool in ["list_blueprints", "list_apps"] {
        let result = call(&server, create_only, tool, json!({})).await;
        assert!(is_error(&result), "{result}");
        assert!(text(&result).contains("`read` scope"), "{result}");
    }
}

// --- 3.3 create_app ----------------------------------------------------------------

/// The app directories under the apps root.
fn app_dirs(state: &AppState) -> Vec<String> {
    std::fs::read_dir(&state.settings.apps.root_folder)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

#[tokio::test]
async fn read_only_connection_cannot_create() {
    let (server, state) = scotty().await;
    let (_client, tokens) = connect(&server, &state, ADMIN, "read", &["read"]).await;
    let read_only = tokens["access_token"].as_str().unwrap();
    let result = call(
        &server,
        read_only,
        "create_app",
        json!({"blueprint": "static-html", "files": [{"path": "html/index.html", "content": "hi"}]}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("`create` scope"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn validation_only_blueprint_is_refused() {
    let (server, state) = scotty().await;
    let result = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "test-oauth-blueprint", "app_name": "x"}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("templated"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn missing_parameter_is_named() {
    let (server, state) = scotty().await;
    let result = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "demo-template", "app_name": "demo"}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("GREETING"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn payload_paths_cannot_escape_the_app() {
    let (server, state) = scotty().await;
    for path in ["../x", "/etc/x", "html/../../x"] {
        let result = call(
            &server,
            ADMIN_TOKEN,
            "create_app",
            json!({"blueprint": "static-html", "files": [{"path": path, "content": "hi"}]}),
        )
        .await;
        assert!(is_error(&result), "{path}: {result}");
    }
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn oversized_payload_states_the_limit() {
    let (server, state) = scotty_with(&[("api.create_app_max_size", "1K")]).await;
    let result = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "static-html", "files": [
            {"path": "html/a.html", "content": "a".repeat(600)},
            // 600 bytes once decoded, 800 as base64.
            {"path": "html/b.bin", "content": BASE64_STANDARD.encode([0u8; 600]), "encoding": "base64"},
        ]}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("limit of 1"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn restricted_user_cannot_create_from_other_blueprints() {
    let (server, state) = scotty().await;
    let result = call(
        &server,
        RESTRICTED_TOKEN,
        "create_app",
        json!({"blueprint": "atelier"}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("any scope"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn existing_name_is_refused_not_overwritten() {
    let (server, state) = scotty().await;
    let dir = std::path::Path::new(&state.settings.apps.root_folder).join("taken");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("marker"), "keep").unwrap();
    let result = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "static-html", "app_name": "taken",
               "files": [{"path": "html/index.html", "content": "hi"}]}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("already exists"), "{result}");
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
}

#[test]
fn generated_names_are_prefixed_and_random() {
    let names: Vec<String> = (0..20)
        .map(|_| crate::api::mcp::generated_name("static-html").unwrap())
        .collect();
    for name in &names {
        let suffix = name.strip_prefix("static-html-").unwrap();
        assert_eq!(suffix.len(), 10, "{name}");
        assert!(
            suffix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "{name}"
        );
    }
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(unique.len(), names.len());
}

// --- 3.4 get_task, destroy_app ---------------------------------------------------------

#[tokio::test]
async fn destroy_is_denied_without_destroy_and_the_app_stays() {
    let (server, state) = scotty().await;
    add_atelier_app(&state).await;

    // The policy refuses: `restricted` may only view in `default`.
    let result = call(
        &server,
        RESTRICTED_TOKEN,
        "destroy_app",
        json!({"app_name": "atelier-demo"}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("Access denied"), "{result}");

    // The token refuses: the admin's connection lacks `destroy`.
    let (_client, tokens) =
        connect(&server, &state, ADMIN, "read create", &["read", "create"]).await;
    let no_destroy = tokens["access_token"].as_str().unwrap();
    let result = call(
        &server,
        no_destroy,
        "destroy_app",
        json!({"app_name": "atelier-demo"}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("`destroy` scope"), "{result}");

    assert!(state.apps.get_app("atelier-demo").await.is_some());
}

#[tokio::test]
async fn hidden_and_unknown_tasks_answer_alike() {
    let (server, state) = scotty().await;
    add_atelier_app(&state).await;
    state
        .auth_service
        .set_app_scopes("atelier-demo", vec!["client-b".into()])
        .await
        .unwrap();
    let (_handle, rx) = state
        .task_manager
        .create_task(scotty_core::tasks::task_details::TaskDetails {
            app_name: Some("atelier-demo".into()),
            ..Default::default()
        })
        .await;
    let id = rx.borrow().id;

    let admin = call(
        &server,
        ADMIN_TOKEN,
        "get_task",
        json!({"task_id": id.to_string()}),
    )
    .await;
    assert!(!is_error(&admin), "{admin}");
    assert_eq!(admin["structuredContent"]["state"], "Running", "{admin}");

    let hidden = call(
        &server,
        RESTRICTED_TOKEN,
        "get_task",
        json!({"task_id": id.to_string()}),
    )
    .await;
    let unknown = call(
        &server,
        RESTRICTED_TOKEN,
        "get_task",
        json!({"task_id": uuid::Uuid::new_v4().to_string()}),
    )
    .await;
    assert!(
        is_error(&hidden) && is_error(&unknown),
        "{hidden} {unknown}"
    );
    assert!(text(&hidden).starts_with("Task not found"), "{hidden}");
    assert!(text(&unknown).starts_with("Task not found"), "{unknown}");
}

#[tokio::test]
async fn tools_list_has_the_six_tools_with_annotations() {
    let (server, _state) = scotty().await;
    let list = rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await;
    assert_eq!(
        tool_names(&list),
        [
            "create_app",
            "destroy_app",
            "get_app",
            "get_task",
            "list_apps",
            "list_blueprints"
        ]
    );
    let tool = |name: &str| {
        list["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == name)
            .unwrap()
            .clone()
    };
    assert_eq!(tool("destroy_app")["annotations"]["destructiveHint"], true);
    assert_eq!(tool("get_task")["annotations"]["readOnlyHint"], true);
    assert_eq!(tool("create_app")["annotations"]["destructiveHint"], false);
}

/// Publish a static page through MCP, poll it to `Finished`, check the files
/// and the app, then destroy it. Needs Docker and the `proxy` network.
/// Run with: cargo test -p scotty --lib publish_poll_and_destroy -- --ignored
#[tokio::test]
#[ignore]
async fn publish_poll_and_destroy_a_static_page() {
    let (server, state) = scotty().await;
    let png = [0x89u8, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0xff];
    let created = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "static-html", "files": [
            {"path": "html/index.html", "content": "<h1>Hällo</h1>"},
            {"path": "html/logo.png", "content": BASE64_STANDARD.encode(png), "encoding": "base64"},
        ]}),
    )
    .await;
    assert!(!is_error(&created), "{created}");
    let created = &created["structuredContent"];
    let name = created["app_name"].as_str().unwrap().to_string();
    let suffix = name.strip_prefix("static-html-").unwrap();
    assert_eq!(suffix.len(), 10, "{name}");
    assert_eq!(
        created["urls"],
        json!([format!("http://web.{name}.test.local")]),
        "{created}"
    );
    let app_dir = std::path::Path::new(&state.settings.apps.root_folder).join(&name);
    let _cleanup = ComposeDown(app_dir.clone());

    let finished = wait(&server, created["task_id"].as_str().unwrap()).await;
    assert_eq!(finished["state"], "Finished", "{finished}");
    assert_eq!(
        std::fs::read(app_dir.join("html/index.html")).unwrap(),
        "<h1>Hällo</h1>".as_bytes()
    );
    assert_eq!(std::fs::read(app_dir.join("html/logo.png")).unwrap(), png);
    let app = call(&server, ADMIN_TOKEN, "get_app", json!({"app_name": name})).await;
    assert_eq!(app["structuredContent"]["status"], "Running", "{app}");

    let destroyed = call(
        &server,
        ADMIN_TOKEN,
        "destroy_app",
        json!({"app_name": name}),
    )
    .await;
    assert!(!is_error(&destroyed), "{destroyed}");
    let task_id = destroyed["structuredContent"]["task_id"].as_str().unwrap();
    assert_eq!(wait(&server, task_id).await["state"], "Finished");
    assert!(!app_dir.exists());
}

/// Poll `get_task` until the task left `Running`.
async fn wait(server: &TestServer, task_id: &str) -> Value {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
    loop {
        let task = call(server, ADMIN_TOKEN, "get_task", json!({"task_id": task_id})).await;
        let task = task["structuredContent"].clone();
        if task["state"] != "Running" {
            return task;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "task did not finish: {task}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

struct ComposeDown(std::path::PathBuf);

impl Drop for ComposeDown {
    fn drop(&mut self) {
        if self.0.exists() {
            let _ = std::process::Command::new("docker")
                .args(["compose", "down", "-v", "--remove-orphans"])
                .current_dir(&self.0)
                .output();
        }
    }
}

// --- Review follow-ups -------------------------------------------------------------

#[tokio::test]
async fn zero_ttl_is_refused() {
    let (server, state) = scotty().await;
    let result = call(
        &server,
        ADMIN_TOKEN,
        "create_app",
        json!({"blueprint": "static-html", "ttl": "0h",
               "files": [{"path": "html/index.html", "content": "hi"}]}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).starts_with("ttl:"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[tokio::test]
async fn policy_without_assignments_refuses_mcp() {
    let dir = tempfile::tempdir().unwrap().keep();
    let policy = std::fs::read_to_string("tests/fixtures/policy/policy.yaml").unwrap();
    let policy = &policy[..policy.find("assignments:").unwrap()];
    std::fs::write(
        dir.join("policy.yaml"),
        format!("{policy}assignments: {{}}\n"),
    )
    .unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings(&[]), None, dir.to_str().unwrap()).await;
    let server = TestServer::new(ApiRoutes::create(state));
    let response = post(
        &server,
        Some(ADMIN_TOKEN),
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
    )
    .await;
    assert_eq!(response.status_code(), 503, "{}", response.text());
}

// --- create_app scope choice ----------------------------------------------------------

fn create_app_schema(list: &Value) -> Value {
    list["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "create_app")
        .unwrap()["inputSchema"]
        .clone()
}

#[tokio::test]
async fn create_app_offers_the_callers_create_scopes() {
    let (server, state) = scotty().await;

    let admin = create_app_schema(&rpc(&server, ADMIN_TOKEN, "tools/list", json!({})).await);
    assert_eq!(
        admin["properties"]["scope"]["enum"],
        json!(["client-a", "client-b", "default"]),
        "{admin}"
    );
    assert_eq!(admin["properties"]["scope"]["type"], "string", "{admin}");
    assert!(!admin["required"]
        .as_array()
        .is_some_and(|r| r.contains(&json!("scope"))));

    let restricted =
        create_app_schema(&rpc(&server, RESTRICTED_TOKEN, "tools/list", json!({})).await);
    assert_eq!(
        restricted["properties"]["scope"]["enum"],
        json!(["default"])
    );

    // `client-a` holds plain `create` in client-a only.
    let client_a = create_app_schema(
        &rpc(
            &server,
            "client-a-secure-token-456",
            "tools/list",
            json!({}),
        )
        .await,
    );
    assert_eq!(client_a["properties"]["scope"]["enum"], json!(["client-a"]));

    // Without the `create` scope there is nothing to offer.
    let (_client, tokens) = connect(&server, &state, ADMIN, "read", &["read"]).await;
    let read_only = tokens["access_token"].as_str().unwrap();
    let schema = create_app_schema(&rpc(&server, read_only, "tools/list", json!({})).await);
    assert!(
        schema["properties"]["scope"].get("enum").is_none(),
        "{schema}"
    );
}

#[tokio::test]
async fn list_blueprints_shows_the_scopes_per_blueprint() {
    let (server, _state) = scotty().await;
    let restricted = call(&server, RESTRICTED_TOKEN, "list_blueprints", json!({})).await;
    assert_eq!(
        restricted["structuredContent"]["blueprints"][0]["scopes"],
        json!(["default"])
    );
}

#[tokio::test]
async fn create_in_a_scope_without_rights_is_refused() {
    let (server, state) = scotty().await;
    let result = call(
        &server,
        RESTRICTED_TOKEN,
        "create_app",
        json!({"blueprint": "static-html", "scope": "client-b",
               "files": [{"path": "html/index.html", "content": "hi"}]}),
    )
    .await;
    assert!(is_error(&result), "{result}");
    assert!(text(&result).contains("Access denied"), "{result}");
    assert!(app_dirs(&state).is_empty());
}

#[test]
fn omitted_scope_resolves_to_the_only_or_default_scope() {
    use crate::api::mcp::default_scope;
    let v = |s: &[&str]| s.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(default_scope(v(&["client-a"])).unwrap(), "client-a");
    assert_eq!(
        default_scope(v(&["client-a", "default"])).unwrap(),
        "default"
    );
    assert!(default_scope(v(&[])).unwrap_err().contains("any scope"));
    let err = default_scope(v(&["client-a", "client-b"])).unwrap_err();
    assert!(err.contains("client-a, client-b"), "{err}");
}
