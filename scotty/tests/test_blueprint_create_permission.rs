//! `create:<blueprint>` lets a user create apps from that templated blueprint
//! only: not from another blueprint, not from a caller compose file. Plain
//! `create` still allows everything.
//!
//! Allowed requests are built to fail validation after the permission check
//! (400), so no app is actually created; denied ones get 403.

use axum_test::TestServer;
use base64::prelude::*;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;

const PUBLISHER: &str = "publisher-token";
const DEVELOPER: &str = "developer-token";

const POLICY: &str = r#"
scopes:
  default:
    description: Default
    created_at: 2024-01-01T00:00:00Z
roles:
  publisher:
    permissions: [view, "create:demo-template"]
    description: May only publish from demo-template
  developer:
    permissions: [view, create]
    description: May create anything
assignments:
  identifier:publisher:
    - { role: publisher, scopes: [default] }
  identifier:developer:
    - { role: developer, scopes: [default] }
"#;

async fn setup() -> (TestServer, tempfile::TempDir) {
    let policy_dir = tempfile::tempdir().unwrap();
    std::fs::copy(
        "tests/fixtures/casbin/model.conf",
        policy_dir.path().join("model.conf"),
    )
    .unwrap();
    std::fs::write(policy_dir.path().join("policy.yaml"), POLICY).unwrap();

    let mut settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .set_override("api.bearer_tokens.publisher", PUBLISHER)
        .unwrap()
        .set_override("api.bearer_tokens.developer", DEVELOPER)
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    scotty_core::settings::app_blueprint::load_blueprint_dirs(
        std::path::Path::new("tests/fixtures/blueprints"),
        &mut settings.apps.blueprints,
    )
    .unwrap();

    let state =
        create_test_app_state_with_policy_dir(settings, None, policy_dir.path().to_str().unwrap())
            .await;
    (TestServer::new(ApiRoutes::create(state)), policy_dir)
}

/// A create request; `files` are (name, content) pairs.
fn request(blueprint: Option<&str>, files: &[(&str, &str)]) -> serde_json::Value {
    serde_json::json!({
        "app_name": "perm-test",
        "settings": {
            "public_services": [],
            "domain": "",
            "time_to_live": "Forever",
            "basic_auth": null,
            "disallow_robots": true,
            "environment": {},
            "registry": null,
            "app_blueprint": blueprint,
        },
        "files": { "files": files.iter().map(|(name, content)| serde_json::json!({
            "name": name, "content": BASE64_STANDARD.encode(content)
        })).collect::<Vec<_>>() },
        "custom_domains": [],
        "requested_scopes": ["default"],
    })
}

async fn status(server: &TestServer, token: &str, body: serde_json::Value) -> u16 {
    server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer(token)
        .json(&body)
        .await
        .status_code()
        .as_u16()
}

const COMPOSE: (&str, &str) = ("docker-compose.yml", "services: {web: {image: x}}");

#[tokio::test]
async fn restricted_user_may_create_from_allowed_blueprint() {
    let (server, _dir) = setup().await;
    // Passes the permission check, then fails on the missing GREETING.
    let response = server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer(PUBLISHER)
        .json(&request(Some("demo-template"), &[]))
        .await;
    assert_eq!(response.status_code(), 400);
    assert!(response.text().contains("GREETING"), "{}", response.text());
}

#[tokio::test]
async fn restricted_user_is_forbidden_elsewhere() {
    let (server, _dir) = setup().await;
    for (case, body) in [
        ("caller compose", request(Some("demo-template"), &[COMPOSE])),
        (
            "caller override file",
            request(
                Some("demo-template"),
                &[("./html/compose.override.yml", "services: {}")],
            ),
        ),
        (
            "other blueprint",
            request(Some("test-blueprint"), &[COMPOSE]),
        ),
        ("no blueprint", request(None, &[COMPOSE])),
    ] {
        assert_eq!(status(&server, PUBLISHER, body).await, 403, "{case}");
    }
}

#[tokio::test]
async fn plain_create_still_allows_everything() {
    let (server, _dir) = setup().await;
    // `.scotty.yml` is rejected right after the permission check.
    let scotty_yml = (".scotty.yml", "x");
    for (case, body) in [
        ("templated", request(Some("demo-template"), &[])),
        (
            "other blueprint",
            request(Some("test-blueprint"), &[COMPOSE, scotty_yml]),
        ),
        ("no blueprint", request(None, &[COMPOSE, scotty_yml])),
    ] {
        assert_eq!(status(&server, DEVELOPER, body).await, 400, "{case}");
    }
}

async fn list(server: &TestServer, token: &str) -> serde_json::Value {
    let response = server
        .get("/api/v1/authenticated/blueprints")
        .authorization_bearer(token)
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    response.json::<serde_json::Value>()["blueprints"].clone()
}

#[tokio::test]
async fn restricted_user_lists_only_allowed_blueprint_with_contract() {
    let (server, _dir) = setup().await;
    let blueprints = list(&server, PUBLISHER).await;
    let names: Vec<_> = blueprints.as_object().unwrap().keys().collect();
    assert_eq!(names, ["demo-template"]);

    let bp = &blueprints["demo-template"];
    assert_eq!(bp["templated"], true);
    let t = &bp["template"];
    assert_eq!(t["images"], serde_json::json!(["alpine:3"]));
    assert_eq!(t["payload"], serde_json::json!(["html/**"]));
    let props = &t["parameters"]["properties"];
    assert_eq!(props["SECRET"]["readOnly"], true);
    assert!(props["GREETING"].get("readOnly").is_none());
    assert!(t.get("files").is_none());
}

#[tokio::test]
async fn plain_create_lists_every_blueprint() {
    let (server, _dir) = setup().await;
    let blueprints = list(&server, DEVELOPER).await;
    assert!(blueprints.get("demo-template").is_some());
    assert_eq!(blueprints["test-blueprint"]["templated"], false);
}

#[tokio::test]
async fn restricted_user_cannot_choose_hostnames() {
    let (server, _dir) = setup().await;
    let mut custom = request(Some("demo-template"), &[]);
    custom["custom_domains"] =
        serde_json::json!([{"domain": "login.example.com", "service": "web"}]);
    let mut services = request(Some("demo-template"), &[]);
    services["settings"]["public_services"] =
        serde_json::json!([{"service": "web", "port": 80, "domains": ["login.example.com"]}]);
    for (case, body) in [("custom domain", custom), ("public services", services)] {
        assert_eq!(
            status(&server, PUBLISHER, body.clone()).await,
            403,
            "{case}"
        );
        // Plain `create` passes the check (and fails later on validation).
        assert_ne!(status(&server, DEVELOPER, body).await, 403, "{case}");
    }
}
