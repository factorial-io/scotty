//! One operation per app: a busy app refuses other exclusive operations with
//! 409 naming the running one, after the permission checks, and shows the
//! running operation in its app data.

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::apps::shared_app_list::AppDataVec;
use serde_json::{json, Value};

/// Bearer tokens of `tests/test_bearer_auth.yaml`.
const ADMIN: &str = "test-bearer-token-123";
const VIEWER: &str = "client-a-secure-token-456";

const POLICY: &str = r#"
scopes:
  ops: { description: Ops, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: d }
  viewer: { permissions: [view], description: d }
assignments:
  identifier:admin:
    - { role: admin, scopes: [ops] }
  identifier:client-a:
    - { role: viewer, scopes: [ops] }
"#;

struct Env {
    server: TestServer,
    state: SharedAppState,
    _policy: tempfile::TempDir,
    _app_dir: tempfile::TempDir,
}

async fn setup() -> Env {
    let policy = tempfile::tempdir().unwrap();
    std::fs::write(policy.path().join("policy.yaml"), POLICY).unwrap();
    let app_dir = tempfile::tempdir().unwrap();
    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .set_override("apps.root_folder", app_dir.path().to_str().unwrap())
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings, None, policy.path().to_str().unwrap())
            .await;
    state
        .auth_service
        .set_app_scopes("shop", vec!["ops".into()])
        .await
        .unwrap();
    state
        .apps
        .add_app(AppData {
            name: "shop".into(),
            root_directory: app_dir.path().to_str().unwrap().into(),
            docker_compose_path: app_dir
                .path()
                .join("docker-compose.yml")
                .to_str()
                .unwrap()
                .into(),
            settings: Some(scotty_core::apps::app_data::AppSettings {
                scopes: vec!["ops".into()],
                ..Default::default()
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    Env {
        server: TestServer::new(ApiRoutes::create(state.clone())),
        state,
        _policy: policy,
        _app_dir: app_dir,
    }
}

fn assert_busy(response: &axum_test::TestResponse, operation: &str) {
    assert_eq!(response.status_code(), 409, "{}", response.text());
    let body: Value = response.json();
    assert_eq!(body["error"], true);
    assert_eq!(body["operation"]["operation"], operation);
    let message = body["message"].as_str().unwrap();
    assert!(
        message.contains("shop") && message.contains(operation),
        "{message}"
    );
}

#[tokio::test]
async fn approving_an_action_during_a_run_is_refused_and_it_stays_pending() {
    let env = setup().await;
    let created = env
        .server
        .post("/api/v1/authenticated/apps/shop/custom-actions")
        .authorization_bearer(ADMIN)
        .json(&json!({
            "name": "migrate",
            "description": "Run migrations",
            "commands": { "web": ["true"] },
            "permission": "action_write"
        }))
        .await;
    assert_eq!(created.status_code(), 201, "{}", created.text());

    let _run = env.state.app_locks.lock("shop", "run").unwrap();
    let approved = env
        .server
        .post("/api/v1/authenticated/admin/actions/shop/migrate/approve")
        .authorization_bearer(ADMIN)
        .json(&json!({ "comment": "ok" }))
        .await;
    assert_busy(&approved, "run");

    let action = env
        .server
        .get("/api/v1/authenticated/apps/shop/custom-actions/migrate")
        .authorization_bearer(ADMIN)
        .await;
    assert_eq!(action.json::<Value>()["status"], "pending");
}

#[tokio::test]
async fn notification_edit_during_a_rebuild_is_refused() {
    let env = setup().await;
    let _rebuild = env.state.app_locks.lock("shop", "rebuild").unwrap();
    let response = env
        .server
        .post("/api/v1/authenticated/apps/notify/add")
        .authorization_bearer(ADMIN)
        .json(&json!({ "app_name": "shop", "service_ids": ["Log"] }))
        .await;
    assert_busy(&response, "rebuild");
}

#[tokio::test]
async fn upload_during_a_rebuild_is_not_refused() {
    let env = setup().await;
    let _rebuild = env.state.app_locks.lock("shop", "rebuild").unwrap();
    let response = env
        .server
        .put("/api/v1/apps/shop/services/web/files")
        .add_query_param("path", "/tmp")
        .authorization_bearer(ADMIN)
        .bytes(Vec::new().into())
        .await;
    // Uploads take no lock; the request may still fail for other reasons (no container).
    assert_ne!(response.status_code(), 409, "{}", response.text());
    assert!(
        !response.text().contains("\"operation\""),
        "{}",
        response.text()
    );
}

#[tokio::test]
async fn run_and_create_during_another_operation_are_refused() {
    let env = setup().await;
    let _create = env.state.app_locks.lock("shop", "create").unwrap();
    let run = env
        .server
        .get("/api/v1/authenticated/apps/run/shop")
        .authorization_bearer(ADMIN)
        .await;
    assert_busy(&run, "create");

    let create = env
        .server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer(ADMIN)
        .json(&json!({
            "app_name": "shop",
            "settings": {
                "public_services": [], "domain": "", "time_to_live": "Forever",
                "basic_auth": null, "disallow_robots": true, "environment": {},
                "registry": null,
            },
            "files": { "files": [] },
            "custom_domains": [],
            "requested_scopes": ["ops"],
        }))
        .await;
    assert_busy(&create, "create");
}

#[tokio::test]
async fn caller_without_permission_gets_403_not_409() {
    let env = setup().await;
    let _rebuild = env.state.app_locks.lock("shop", "rebuild").unwrap();
    let stop = env
        .server
        .get("/api/v1/authenticated/apps/stop/shop")
        .authorization_bearer(VIEWER)
        .await;
    assert_eq!(stop.status_code(), 403, "{}", stop.text());
    let approve = env
        .server
        .post("/api/v1/authenticated/admin/actions/shop/migrate/approve")
        .authorization_bearer(VIEWER)
        .json(&json!({}))
        .await;
    assert_eq!(approve.status_code(), 403, "{}", approve.text());
}

#[tokio::test]
async fn app_info_shows_the_running_operation_until_it_ends() {
    let env = setup().await;
    let info = || async {
        env.server
            .get("/api/v1/authenticated/apps/info/shop")
            .authorization_bearer(VIEWER)
            .await
            .json::<Value>()
    };
    assert_eq!(info().await["operation"], Value::Null);

    let task_id = uuid::Uuid::new_v4();
    let mut rebuild = env.state.app_locks.lock("shop", "rebuild").unwrap();
    rebuild.set_task(task_id, env.state.clone());
    let shown = info().await;
    assert_eq!(shown["operation"]["operation"], "rebuild");
    assert_eq!(shown["operation"]["task_id"], task_id.to_string());

    // The scheduled sweep replaces the app list; the operation comes from the
    // lock and survives it.
    let apps = env.state.apps.get_apps().await;
    env.state
        .apps
        .set_apps(&AppDataVec { apps: apps.apps })
        .await
        .unwrap();
    assert_eq!(info().await["operation"]["operation"], "rebuild");
    let list = env
        .server
        .get("/api/v1/authenticated/apps/list")
        .authorization_bearer(VIEWER)
        .await
        .json::<Value>();
    assert_eq!(list["apps"][0]["operation"]["operation"], "rebuild");

    drop(rebuild);
    assert_eq!(info().await["operation"], Value::Null);
}
