//! A templated blueprint creates an app without a caller compose file, and
//! the app directory is its snapshot: rebuilds ignore later blueprint changes.
//!
//! Marked #[ignore] because it drives a real `docker compose up`.
//! Run locally with: cargo test --test test_blueprint_template_create -- --ignored --nocapture

use std::time::{Duration, Instant};

use axum_test::TestServer;
use base64::prelude::*;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_settings;
use scotty::docker::loadbalancer::app_proxy_network_name;
use scotty_types::State;

const ADMIN_TOKEN: &str = "test-bearer-token-123";

#[tokio::test]
#[ignore]
async fn templated_create_writes_merged_files_and_rebuild_keeps_snapshot() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    #[cfg(any(feature = "telemetry-grpc", feature = "telemetry-http"))]
    scotty::metrics::init_metrics().expect("failed to initialise metrics");
    let root = std::env::temp_dir().join(format!("scotty-bp-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    // Settings::new canonicalizes root_folder; on macOS temp_dir is a symlink.
    let root = std::fs::canonicalize(&root).unwrap();

    let config = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .build()
        .unwrap();
    let mut settings: scotty::settings::config::Settings = config.try_deserialize().unwrap();
    settings.apps.root_folder = root.to_str().unwrap().to_string();
    scotty_core::settings::app_blueprint::load_blueprint_dirs(
        std::path::Path::new("tests/fixtures/blueprints"),
        &mut settings.apps.blueprints,
    )
    .unwrap();
    let app_name = "bp-template";
    let app_dir = root.join(app_name);
    let _cleanup = Cleanup {
        app_dir: app_dir.clone(),
        root: root.clone(),
        network: app_proxy_network_name(&settings.traefik.network, app_name),
    };

    let app_state = create_test_app_state_with_settings(settings, None).await;
    let server = TestServer::new(ApiRoutes::create(app_state.clone()));

    let response = server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer(ADMIN_TOKEN)
        .json(&serde_json::json!({
            "app_name": app_name,
            "settings": {
                "public_services": [],
                "domain": "",
                "time_to_live": "Forever",
                "basic_auth": null,
                "disallow_robots": true,
                "environment": {},
                "registry": null,
                "app_blueprint": "demo-template",
            },
            "files": { "files": [
                { "name": "html/index.html", "content": BASE64_STANDARD.encode("<h1>hi</h1>") }
            ]},
            "custom_domains": [],
            "blueprint_parameters": { "GREETING": "hello" },
        }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let task = wait_for_task(&server, &task_id(&response.json())).await;
    task.assert_finished(&app_state).await;

    let compose_path = app_dir.join("compose.yml");
    let original = std::fs::read_to_string(&compose_path).unwrap();
    assert!(original.contains("${IMAGE}"));
    assert_eq!(
        std::fs::read_to_string(app_dir.join("html/index.html")).unwrap(),
        "<h1>hi</h1>"
    );
    let scotty_yml = std::fs::read_to_string(app_dir.join(".scotty.yml")).unwrap();
    assert!(scotty_yml.contains("blueprint_hash"), "{scotty_yml}");
    assert!(scotty_yml.contains("generated_parameters"), "{scotty_yml}");

    // Settings are immutable at runtime, so "the blueprint changed" means a
    // restart. Simulate the effect on disk: rebuild must use the app dir only.
    let snapshot = original.replace("sleep 3600", "sleep 3601");
    std::fs::write(&compose_path, &snapshot).unwrap();

    let response = server
        .get(&format!("/api/v1/authenticated/apps/rebuild/{app_name}"))
        .authorization_bearer(ADMIN_TOKEN)
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let task = wait_for_task(&server, &task_id(&response.json())).await;
    task.assert_finished(&app_state).await;
    assert_eq!(std::fs::read_to_string(&compose_path).unwrap(), snapshot);
}

/// The HTTP task view omits output lines; keep the state and print the
/// status line from the task manager on failure.
struct TaskResult(scotty_types::TaskDetails);

impl TaskResult {
    async fn assert_finished(self, app_state: &scotty::app_state::SharedAppState) {
        if self.0.state == State::Finished {
            return;
        }
        let details = app_state.task_manager.get_task_details(&self.0.id).await;
        panic!(
            "task {:?}: {:#?}",
            self.0.state,
            details.map(|d| d.output.lines)
        );
    }
}

fn task_id(v: &serde_json::Value) -> String {
    v["task"]["id"].as_str().unwrap().to_string()
}

async fn wait_for_task(server: &TestServer, task_id: &str) -> TaskResult {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let task: scotty_types::TaskDetails = server
            .get(&format!("/api/v1/authenticated/task/{task_id}"))
            .authorization_bearer(ADMIN_TOKEN)
            .await
            .json();
        if task.state != State::Running {
            return TaskResult(task);
        }
        assert!(Instant::now() < deadline, "task did not finish");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

struct Cleanup {
    app_dir: std::path::PathBuf,
    root: std::path::PathBuf,
    network: String,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::process::Command::new("docker")
            .args(["compose", "down", "-v", "--remove-orphans"])
            .current_dir(&self.app_dir)
            .output();
        let _ = std::process::Command::new("docker")
            .args(["network", "rm", &self.network])
            .output();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
