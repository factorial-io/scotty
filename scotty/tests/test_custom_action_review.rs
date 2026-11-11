//! Creating and reviewing a custom action records the caller from the
//! authorization context as creator and reviewer.

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty_core::apps::app_data::AppData;
use serde_json::{json, Value};

const USER: &str = "reviewer@example.com";

const POLICY: &str = r#"
scopes:
  ops: { description: Ops, created_at: 2024-01-01T00:00:00Z }
roles:
  action_admin: { permissions: [action_manage, action_approve], description: d }
assignments:
  reviewer@example.com:
    - { role: action_admin, scopes: [ops] }
"#;

#[tokio::test]
async fn creator_and_reviewer_are_the_calling_user() {
    let policy_dir = tempfile::tempdir().unwrap();
    std::fs::write(policy_dir.path().join("policy.yaml"), POLICY).unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.auth_mode", "dev")
        .unwrap()
        .set_override("api.dev_user_email", USER)
        .unwrap()
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings, None, policy_dir.path().to_str().unwrap())
            .await;
    state
        .auth_service
        .set_app_scopes("ops-app", vec!["ops".into()])
        .await
        .unwrap();
    state
        .apps
        .add_app(AppData {
            name: "ops-app".into(),
            root_directory: app_dir.path().to_str().unwrap().into(),
            settings: Some(Default::default()),
            ..Default::default()
        })
        .await
        .unwrap();
    let server = TestServer::new(ApiRoutes::create(state));

    let created = server
        .post("/api/v1/authenticated/apps/ops-app/custom-actions")
        .json(&json!({
            "name": "migrate",
            "description": "Run migrations",
            "commands": { "web": ["true"] },
            "permission": "action_write"
        }))
        .await;
    assert_eq!(created.status_code(), 201, "{}", created.text());
    assert_eq!(created.json::<Value>()["created_by"], USER);

    let approved = server
        .post("/api/v1/authenticated/admin/actions/ops-app/migrate/approve")
        .json(&json!({ "comment": "ok" }))
        .await;
    assert_eq!(approved.status_code(), 200, "{}", approved.text());
    let action: Value = approved.json();
    assert_eq!(action["reviewed_by"], USER);
    assert_eq!(action["status"], "approved");
}
