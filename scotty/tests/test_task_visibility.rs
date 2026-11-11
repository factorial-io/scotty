//! Task list and task detail only return tasks the caller may see: the user
//! who started the task; for a task of an app, `admin_read` on that app, the
//! action's own permission for action runs, `view` for other tasks; for a task
//! without an app, `admin_read`.

use std::path::Path;

use axum_test::TestServer;
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use scotty::app_state::SharedAppState;
use scotty::services::authorization::types::PermissionOrWildcard;
use scotty::services::authorization::Permission;
use scotty_types::TaskDetails;
use uuid::Uuid;

const ADMIN_TOKEN: &str = "test-bearer-token-123";
/// `identifier:client-a` has `developer` in scope `client-a` only: `view`,
/// but no action permissions.
const CLIENT_A_TOKEN: &str = "client-a-secure-token-456";
/// `identifier:no-rbac` gets `action_read` in scope `client-a` in [`setup`].
const ACTION_READER_TOKEN: &str = "token-without-rbac-assignments";
/// Has only `admin_read` (in `default`), no role in `client-a` or `client-b`.
const AUDITOR: &str = "auditor@example.com";
/// Has only `admin_read`, in `client-a`.
const AUDITOR_A: &str = "auditor-a@example.com";

struct Tasks {
    in_a: Uuid,
    in_b: Uuid,
    no_app: Uuid,
    started_by_a: Uuid,
    /// `drush:uli`-like action run in app-a by someone else.
    write_action_in_a: Uuid,
    read_action_in_a: Uuid,
    action_started_by_a: Uuid,
    bogus_action_in_a: Uuid,
}

async fn setup() -> (TestServer, SharedAppState, Tasks, tempfile::TempDir) {
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

    let auth = &state.auth_service;
    for (role, permission, user, scope) in [
        (
            "action-reader",
            Permission::ActionRead,
            "identifier:no-rbac",
            "client-a",
        ),
        ("auditor", Permission::AdminRead, AUDITOR, "default"),
    ] {
        auth.create_role(
            role,
            vec![PermissionOrWildcard::Permission(permission)],
            role,
        )
        .await
        .unwrap();
        auth.assign_user_role(user, role, vec![scope.to_string()])
            .await
            .unwrap();
    }
    auth.assign_user_role(AUDITOR_A, "auditor", vec!["client-a".to_string()])
        .await
        .unwrap();

    let create = |app: Option<&str>, requested_by: Option<&str>, action: Option<&str>| {
        let state = state.clone();
        let details = TaskDetails {
            app_name: app.map(str::to_string),
            requested_by: requested_by.map(str::to_string),
            action_permission: action.map(str::to_string),
            ..TaskDetails::default()
        };
        async move { state.task_manager.create_task(details).await.1.borrow().id }
    };
    let someone = Some("identifier:admin");
    let tasks = Tasks {
        in_a: create(Some("app-a"), None, None).await,
        in_b: create(Some("app-b"), None, None).await,
        no_app: create(None, None, None).await,
        // App not assigned to any scope yet, as during app creation.
        started_by_a: create(Some("app-new"), Some("identifier:client-a"), None).await,
        write_action_in_a: create(Some("app-a"), someone, Some("action_write")).await,
        read_action_in_a: create(Some("app-a"), someone, Some("action_read")).await,
        action_started_by_a: create(
            Some("app-a"),
            Some("identifier:client-a"),
            Some("action_write"),
        )
        .await,
        bogus_action_in_a: create(Some("app-a"), someone, Some("no-such-permission")).await,
    };

    (
        TestServer::new(ApiRoutes::create(state.clone())),
        state,
        tasks,
        policy_dir,
    )
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
    let (server, _state, t, _dir) = setup().await;

    let admin = listed(&server, ADMIN_TOKEN).await;
    for id in [
        t.in_a,
        t.in_b,
        t.no_app,
        t.write_action_in_a,
        t.bogus_action_in_a,
    ] {
        assert!(admin.contains(&id), "admin should see {id}");
    }

    // `view` in client-a: app tasks and own runs, but no other user's actions.
    let mut client_a = listed(&server, CLIENT_A_TOKEN).await;
    client_a.sort();
    let mut expected = vec![t.in_a, t.started_by_a, t.action_started_by_a];
    expected.sort();
    assert_eq!(client_a, expected);

    // `action_read` in client-a, no `view`: only the read-only action.
    assert_eq!(
        listed(&server, ACTION_READER_TOKEN).await,
        vec![t.read_action_in_a]
    );
}

#[tokio::test]
async fn action_tasks_need_the_action_permission() {
    let (_server, state, t, _dir) = setup().await;
    let task = |id: Uuid| {
        let state = state.clone();
        async move { state.task_manager.get_task_details(&id).await.unwrap() }
    };
    let can = |user: &'static str, id: Uuid| {
        let state = state.clone();
        async move {
            let details = task(id).await;
            state.auth_service.can_view_task(user, &details).await
        }
    };

    // `view` alone doesn't reveal another user's action.
    assert!(!can("identifier:client-a", t.write_action_in_a).await);
    assert!(!can("identifier:client-a", t.read_action_in_a).await);
    // The requester always sees their own run.
    assert!(can("identifier:client-a", t.action_started_by_a).await);
    // The action's own permission is enough, and nothing more is needed.
    assert!(can("identifier:no-rbac", t.read_action_in_a).await);
    assert!(!can("identifier:no-rbac", t.write_action_in_a).await);
    // `view` still covers non-action tasks.
    assert!(can("identifier:client-a", t.in_a).await);
    // An unknown permission hides the task (fail closed), except for admins.
    assert!(!can("identifier:client-a", t.bogus_action_in_a).await);
    assert!(can("identifier:admin", t.bogus_action_in_a).await);
    // `admin_read` outside the app's scopes doesn't reveal the app's tasks,
    // only tasks without an app.
    for id in [t.in_a, t.write_action_in_a, t.in_b, t.bogus_action_in_a] {
        assert!(!can(AUDITOR, id).await, "auditor should not see {id}");
    }
    assert!(can(AUDITOR, t.no_app).await);
    // `admin_read` in the app's scope reveals all of the app's tasks.
    for id in [
        t.in_a,
        t.write_action_in_a,
        t.read_action_in_a,
        t.bogus_action_in_a,
        t.no_app,
    ] {
        assert!(can(AUDITOR_A, id).await, "auditor-a should see {id}");
    }
    assert!(!can(AUDITOR_A, t.in_b).await);
}

#[tokio::test]
async fn hidden_task_detail_answers_not_found() {
    let (server, _state, t, _dir) = setup().await;

    assert_eq!(
        detail_status(&server, CLIENT_A_TOKEN, t.write_action_in_a).await,
        404
    );
    assert_eq!(
        detail_status(&server, CLIENT_A_TOKEN, t.action_started_by_a).await,
        200
    );
    assert_eq!(
        detail_status(&server, ADMIN_TOKEN, t.write_action_in_a).await,
        200
    );

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
