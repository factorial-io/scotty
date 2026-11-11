//! Task list and task detail only return tasks the caller may see: `view` on
//! the task's app, `admin_read` for tasks without an app, or the user who
//! started the task.

use std::path::Path;

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty_types::TaskDetails;
use uuid::Uuid;

const ADMIN_TOKEN: &str = "test-bearer-token-123";
/// `identifier:client-a` has `developer` in scope `client-a` only.
const CLIENT_A_TOKEN: &str = "client-a-secure-token-456";

struct Tasks {
    in_a: Uuid,
    in_b: Uuid,
    no_app: Uuid,
    started_by_a: Uuid,
}

async fn setup() -> (TestServer, Tasks, tempfile::TempDir) {
    let policy_dir = tempfile::tempdir().unwrap();
    for file in ["model.conf", "policy.yaml"] {
        std::fs::copy(
            Path::new("tests/fixtures/casbin").join(file),
            policy_dir.path().join(file),
        )
        .unwrap();
    }
    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let state =
        create_test_app_state_with_policy_dir(settings, None, policy_dir.path().to_str().unwrap())
            .await;

    for (app, scope) in [("app-a", "client-a"), ("app-b", "client-b")] {
        state
            .auth_service
            .set_app_scopes(app, vec![scope.to_string()])
            .await
            .unwrap();
    }

    let create = |app: Option<&str>, requested_by: Option<&str>| {
        let state = state.clone();
        let details = TaskDetails {
            app_name: app.map(str::to_string),
            requested_by: requested_by.map(str::to_string),
            ..TaskDetails::default()
        };
        async move { state.task_manager.create_task(details).await.1.borrow().id }
    };
    let tasks = Tasks {
        in_a: create(Some("app-a"), None).await,
        in_b: create(Some("app-b"), None).await,
        no_app: create(None, None).await,
        // App not assigned to any scope yet, as during app creation.
        started_by_a: create(Some("app-new"), Some("identifier:client-a")).await,
    };

    (TestServer::new(ApiRoutes::create(state)), tasks, policy_dir)
}

async fn listed(server: &TestServer, token: &str) -> Vec<Uuid> {
    let response = server
        .get("/api/v1/authenticated/tasks")
        .authorization_bearer(token)
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    response.json::<serde_json::Value>()["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap().parse().unwrap())
        .collect()
}

async fn detail_status(server: &TestServer, token: &str, id: Uuid) -> u16 {
    server
        .get(&format!("/api/v1/authenticated/task/{id}"))
        .authorization_bearer(token)
        .await
        .status_code()
        .as_u16()
}

#[tokio::test]
async fn task_list_is_filtered_per_caller() {
    let (server, t, _dir) = setup().await;

    let admin = listed(&server, ADMIN_TOKEN).await;
    for id in [t.in_a, t.in_b, t.no_app] {
        assert!(admin.contains(&id), "admin should see {id}");
    }

    let mut client_a = listed(&server, CLIENT_A_TOKEN).await;
    client_a.sort();
    let mut expected = vec![t.in_a, t.started_by_a];
    expected.sort();
    assert_eq!(client_a, expected);
}

#[tokio::test]
async fn hidden_task_detail_answers_not_found() {
    let (server, t, _dir) = setup().await;

    assert_eq!(detail_status(&server, CLIENT_A_TOKEN, t.in_a).await, 200);
    assert_eq!(
        detail_status(&server, CLIENT_A_TOKEN, t.started_by_a).await,
        200
    );
    assert_eq!(detail_status(&server, CLIENT_A_TOKEN, t.in_b).await, 404);
    assert_eq!(detail_status(&server, CLIENT_A_TOKEN, t.no_app).await, 404);
    assert_eq!(detail_status(&server, ADMIN_TOKEN, t.no_app).await, 200);
    // Same answer as for a task that does not exist.
    assert_eq!(
        detail_status(&server, CLIENT_A_TOKEN, Uuid::new_v4()).await,
        404
    );
}
