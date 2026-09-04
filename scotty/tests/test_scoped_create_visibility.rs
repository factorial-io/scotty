//! Regression test for factorial-io/scotty#894: an app created with a scoped
//! (non-admin) bearer token must be visible to that same token as soon as the
//! create task reports completion, not only after the next scheduled app scan.
//!
//! Marked #[ignore] because it drives a real `docker compose up`.
//! Run locally with: cargo test --test test_scoped_create_visibility -- --ignored --nocapture

use std::time::{Duration, Instant};

use axum_test::TestServer;
use base64::prelude::*;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_settings;
use scotty::docker::loadbalancer::app_proxy_network_name;
use scotty_types::State;

/// `identifier:client-a` has the `developer` role in scope `client-a` only.
const SCOPED_TOKEN: &str = "client-a-secure-token-456";

#[tokio::test]
#[ignore]
async fn scoped_token_can_read_app_it_just_created() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
    let root = std::env::temp_dir().join(format!("scotty-894-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();

    let config = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .build()
        .unwrap();
    let mut settings: scotty::settings::config::Settings = config.try_deserialize().unwrap();
    settings.apps.root_folder = root.to_str().unwrap().to_string();
    let app_name = "issue-894";
    // Drop guard so containers, network and temp dir go away on any panic.
    let _cleanup = Cleanup {
        app_dir: root.join(app_name),
        root: root.clone(),
        network: app_proxy_network_name(&settings.traefik.network, app_name),
    };

    let app_state = create_test_app_state_with_settings(settings, None).await;
    let server = TestServer::new(ApiRoutes::create(app_state.clone()));

    let compose = "services:\n  web:\n    image: alpine:3\n    command: sleep 3600\n";

    let response = server
        .post("/api/v1/authenticated/apps/create")
        .authorization_bearer(SCOPED_TOKEN)
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
                "app_blueprint": null,
            },
            "files": { "files": [
                { "name": "docker-compose.yml", "content": BASE64_STANDARD.encode(compose) }
            ]},
            "custom_domains": [],
            "requested_scopes": ["client-a"],
        }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let task_id = response.json::<serde_json::Value>()["task"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Poll quickly: before the fix the task flipped to Finished after each
    // subprocess, so the window in which apps/info fails is short.
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let task: scotty_types::TaskDetails = server
            .get(&format!("/api/v1/authenticated/task/{task_id}"))
            .authorization_bearer(SCOPED_TOKEN)
            .await
            .json();
        match task.state {
            State::Finished => break,
            State::Failed => panic!("create task failed"),
            State::Running => {
                assert!(Instant::now() < deadline, "create task did not finish");
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    }

    let info = server
        .get(&format!("/api/v1/authenticated/apps/info/{app_name}"))
        .authorization_bearer(SCOPED_TOKEN)
        .await;

    assert_eq!(info.status_code(), 200, "{}", info.text());
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
