//! Create is create-or-update: re-creating an existing app is authorized
//! against the app on disk (openspec change create-existing-app).
//!
//! Requests that pass authorization carry a `.scotty.yml` file (or miss a
//! required parameter), so they fail with 400 right after the checks and
//! nothing is actually created; denied ones get 403.

use axum_test::TestServer;
use base64::prelude::*;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;

const PUBLISHER: &str = "publisher-token";
const DEVELOPER: &str = "developer-token";
const OPS: &str = "ops-token";

const POLICY: &str = r#"
scopes:
  default: { description: Default, created_at: 2024-01-01T00:00:00Z }
  production: { description: Production, created_at: 2024-01-01T00:00:00Z }
roles:
  publisher: { permissions: [view, "create:demo-template"], description: x }
  developer: { permissions: [view, create], description: x }
assignments:
  identifier:publisher:
    - { role: publisher, scopes: [default] }
  identifier:developer:
    - { role: developer, scopes: [default] }
  identifier:ops:
    - { role: developer, scopes: [default, production] }
"#;

struct Env {
    server: TestServer,
    _policy: tempfile::TempDir,
    apps: tempfile::TempDir,
}

impl Env {
    /// An existing app directory; `settings` is the `.scotty.yml` content.
    fn existing(&self, name: &str, settings: Option<&str>) {
        let dir = self.apps.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        if let Some(s) = settings {
            std::fs::write(dir.join(".scotty.yml"), s).unwrap();
        }
    }

    async fn status(&self, token: &str, body: serde_json::Value) -> (u16, String) {
        let r = self
            .server
            .post("/api/v1/authenticated/apps/create")
            .authorization_bearer(token)
            .json(&body)
            .await;
        (r.status_code().as_u16(), r.text())
    }
}

async fn setup() -> Env {
    let policy = tempfile::tempdir().unwrap();
    let apps = tempfile::tempdir().unwrap();
    std::fs::copy(
        "tests/fixtures/casbin/model.conf",
        policy.path().join("model.conf"),
    )
    .unwrap();
    std::fs::write(policy.path().join("policy.yaml"), POLICY).unwrap();

    let mut settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .set_override("apps.root_folder", apps.path().to_str().unwrap())
        .unwrap()
        .set_override("api.bearer_tokens.publisher", PUBLISHER)
        .unwrap()
        .set_override("api.bearer_tokens.developer", DEVELOPER)
        .unwrap()
        .set_override("api.bearer_tokens.ops", OPS)
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
        create_test_app_state_with_policy_dir(settings, None, policy.path().to_str().unwrap())
            .await;
    Env {
        server: TestServer::new(ApiRoutes::create(state)),
        _policy: policy,
        apps,
    }
}

/// A create request. Without a blueprint it carries a compose file plus a
/// `.scotty.yml`, which is rejected (400) right after authorization.
fn request(name: &str, blueprint: Option<&str>, scopes: &[&str]) -> serde_json::Value {
    let files: Vec<(&str, &str)> = match blueprint {
        Some(_) => vec![],
        None => vec![
            ("docker-compose.yml", "services: {web: {image: x}}"),
            (".scotty.yml", "x"),
        ],
    };
    serde_json::json!({
        "app_name": name,
        "settings": {
            "public_services": [], "domain": "", "time_to_live": "Forever",
            "basic_auth": null, "disallow_robots": true, "environment": {},
            "registry": null, "app_blueprint": blueprint,
        },
        "files": { "files": files.iter().map(|(n, c)| serde_json::json!({
            "name": n, "content": BASE64_STANDARD.encode(c)
        })).collect::<Vec<_>>() },
        "custom_domains": [],
        "requested_scopes": scopes,
    })
}

fn settings_yml(scopes: &[&str], blueprint: Option<&str>) -> String {
    let s = scotty_core::apps::app_data::AppSettings {
        scopes: scopes.iter().map(|s| s.to_string()).collect(),
        app_blueprint: blueprint.map(str::to_string),
        ..Default::default()
    };
    serde_norway::to_string(&s).unwrap()
}

#[tokio::test]
async fn re_create_own_app_is_allowed() {
    let env = setup().await;
    env.existing("shop", Some(&settings_yml(&["default"], None)));
    let (code, _) = env
        .status(DEVELOPER, request("shop", None, &["default"]))
        .await;
    assert_eq!(
        code, 400,
        "passes authorization, then hits the .scotty.yml check"
    );
}

#[tokio::test]
async fn overwrite_across_scopes_is_forbidden() {
    let env = setup().await;
    env.existing("shop", Some(&settings_yml(&["production"], None)));
    let (code, text) = env
        .status(DEVELOPER, request("shop", None, &["default"]))
        .await;
    assert_eq!(code, 403, "{text}");
}

/// Rights in one scope of a multi-scope app aren't enough to replace it.
#[tokio::test]
async fn re_create_needs_rights_in_every_scope_of_the_app() {
    let env = setup().await;
    env.existing(
        "both",
        Some(&settings_yml(&["default", "production"], None)),
    );
    let body = request("both", None, &["default", "production"]);
    // `developer` has create in `default` only.
    let (code, text) = env.status(DEVELOPER, body.clone()).await;
    assert_eq!(code, 403, "{text}");
    // `ops` has create in both.
    let (code, text) = env.status(OPS, body).await;
    assert_eq!(code, 400, "{text}");
}

#[tokio::test]
async fn scope_change_on_re_create_is_forbidden() {
    let env = setup().await;
    env.existing("shop", Some(&settings_yml(&["production"], None)));
    let (code, text) = env.status(OPS, request("shop", None, &["default"])).await;
    assert_eq!(code, 403, "{text}");
    assert!(text.contains("can't change"), "{text}");

    // Same scopes in another order are not a change.
    env.existing(
        "both",
        Some(&settings_yml(&["default", "production"], None)),
    );
    let (code, _) = env
        .status(OPS, request("both", None, &["production", "default"]))
        .await;
    assert_eq!(code, 400);
}

#[tokio::test]
async fn blueprint_scoped_create_only_updates_apps_from_that_blueprint() {
    let env = setup().await;
    env.existing("notes", Some(&settings_yml(&["default"], None)));
    env.existing(
        "report",
        Some(&settings_yml(&["default"], Some("demo-template"))),
    );

    let (code, text) = env
        .status(
            PUBLISHER,
            request("notes", Some("demo-template"), &["default"]),
        )
        .await;
    assert_eq!(code, 403, "{text}");

    // Passes authorization, then fails on the missing GREETING parameter.
    let (code, text) = env
        .status(
            PUBLISHER,
            request("report", Some("demo-template"), &["default"]),
        )
        .await;
    assert_eq!(code, 400, "{text}");
    assert!(text.contains("GREETING"), "{text}");
}

#[tokio::test]
async fn directory_without_settings_counts_as_default_scope() {
    let env = setup().await;
    env.existing("legacy", None);
    let (code, _) = env
        .status(
            PUBLISHER,
            request("legacy", Some("demo-template"), &["default"]),
        )
        .await;
    assert_eq!(
        code, 403,
        "no blueprint recorded, so create:<bp> doesn't cover it"
    );
    let (code, _) = env
        .status(DEVELOPER, request("legacy", None, &["default"]))
        .await;
    assert_eq!(code, 400);
}

#[tokio::test]
async fn unreadable_settings_reject_the_request() {
    let env = setup().await;
    env.existing("broken", Some(":\n- not: [settings"));
    let (code, text) = env
        .status(DEVELOPER, request("broken", None, &["default"]))
        .await;
    // A broken file on the server is a server error, and its contents
    // stay out of the response.
    assert_eq!(code, 500);
    assert!(!text.contains("not: [settings"), "{text}");
}
