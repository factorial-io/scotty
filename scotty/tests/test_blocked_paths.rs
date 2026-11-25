//! App and blueprint responses show blocked paths: an app its effective
//! list (global plus blueprint, read from the current config), the blueprint
//! listing each blueprint's own list and the global list once.

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty_core::apps::app_data::{AppData, AppSettings};
use serde_json::{json, Value};

/// Bearer token of `tests/test_bearer_auth.yaml` mapped to `identifier:admin`.
const ADMIN: &str = "test-bearer-token-123";

const POLICY: &str = r#"
scopes:
  ops: { description: Ops, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: d }
assignments:
  identifier:admin:
    - { role: admin, scopes: [ops] }
"#;

#[tokio::test]
async fn app_and_blueprint_responses_show_blocked_paths() {
    let policy = tempfile::tempdir().unwrap();
    std::fs::write(policy.path().join("policy.yaml"), POLICY).unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    let mut settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .set_override("apps.root_folder", app_dir.path().to_str().unwrap())
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    settings.apps.blocked_paths = vec!["/core/install.php".into(), "/.git/".into()];
    settings.apps.blueprints.insert(
        "drupal".into(),
        serde_norway::from_str(
            "{name: Drupal, description: d, actions: {}, required_services: [], \
             blocked_paths: [/core/rebuild.php, /core/install.php]}",
        )
        .unwrap(),
    );
    let state =
        create_test_app_state_with_policy_dir(settings, None, policy.path().to_str().unwrap())
            .await;

    for (name, blueprint) in [
        ("cms", Some("drupal")),
        ("shop", None),
        ("gone", Some("old")),
    ] {
        state
            .auth_service
            .set_app_scopes(name, vec!["ops".into()])
            .await
            .unwrap();
        state
            .apps
            .add_app(AppData {
                name: name.into(),
                settings: Some(AppSettings {
                    scopes: vec!["ops".into()],
                    app_blueprint: blueprint.map(Into::into),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .await
            .unwrap();
    }
    let server = TestServer::new(ApiRoutes::create(state));

    let info = |app: &'static str| {
        let server = &server;
        async move {
            server
                .get(&format!("/api/v1/authenticated/apps/info/{app}"))
                .authorization_bearer(ADMIN)
                .await
                .json::<Value>()
        }
    };
    assert_eq!(
        info("cms").await["blocked_paths"],
        json!(["/.git/", "/core/install.php", "/core/rebuild.php"])
    );
    assert_eq!(
        info("shop").await["blocked_paths"],
        json!(["/.git/", "/core/install.php"])
    );
    // A blueprint no longer on the server contributes nothing.
    assert_eq!(
        info("gone").await["blocked_paths"],
        json!(["/.git/", "/core/install.php"])
    );

    let list: Value = server
        .get("/api/v1/authenticated/apps/list")
        .authorization_bearer(ADMIN)
        .await
        .json();
    let cms = list["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "cms")
        .unwrap();
    assert_eq!(
        cms["blocked_paths"],
        json!(["/.git/", "/core/install.php", "/core/rebuild.php"])
    );

    let blueprints: Value = server
        .get("/api/v1/authenticated/blueprints")
        .authorization_bearer(ADMIN)
        .await
        .json();
    assert_eq!(
        blueprints["global_blocked_paths"],
        json!(["/core/install.php", "/.git/"])
    );
    assert_eq!(
        blueprints["blueprints"]["drupal"]["blocked_paths"],
        json!(["/core/rebuild.php", "/core/install.php"])
    );
}
