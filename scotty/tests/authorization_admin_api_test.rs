//! The admin API refuses policy changes that would not load with 4xx and a
//! message naming the offending value, and leaves `policy.yaml` unchanged.

use axum_test::{TestResponse, TestServer};
use scotty::api::router::ApiRoutes;
use scotty::api::test_utils::create_test_app_state_with_policy_dir;
use serde_json::{json, Value};

const ADMIN: &str = "admin@example.com";

const POLICY: &str = r#"
scopes:
  staging: { description: Staging, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: Admin }
  developer: { permissions: [view, manage], description: Developer }
assignments:
  admin@example.com:
    - { role: admin, scopes: ['*'] }
"#;

/// A server in dev auth mode acting as [`ADMIN`], and the policy file it
/// writes to.
async fn admin_server() -> (TestServer, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap().keep();
    std::fs::write(dir.join("policy.yaml"), POLICY).unwrap();
    let settings: scotty::settings::config::Settings = config::Config::builder()
        .add_source(config::File::with_name("tests/test_bearer_auth"))
        .set_override("api.auth_mode", "dev")
        .unwrap()
        .set_override("api.dev_user_email", ADMIN)
        .unwrap()
        .set_override("api.rate_limiting.enabled", false)
        .unwrap()
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    let state = create_test_app_state_with_policy_dir(settings, None, dir.to_str().unwrap()).await;
    // The state works on a copy of `dir`.
    let policy = std::path::PathBuf::from(state.auth_service.config_path());
    (TestServer::new(ApiRoutes::create(state)), policy)
}

fn message(response: &TestResponse) -> String {
    let body: Value = response.json();
    assert_eq!(body["error"], true, "{body}");
    body["message"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn role_created_via_api_is_saved_and_loads_again() {
    let (server, policy) = admin_server().await;
    let response = server
        .post("/api/v1/authenticated/admin/roles")
        .json(&json!({ "name": "auditor", "permissions": ["view"], "description": "Audit" }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());

    let written = std::fs::read_to_string(&policy).unwrap();
    assert!(written.contains("auditor"), "{written}");
    let dir = policy.parent().unwrap().to_str().unwrap();
    let reloaded = scotty::services::AuthorizationService::new(dir)
        .await
        .unwrap();
    assert!(reloaded
        .list_roles()
        .await
        .iter()
        .any(|(name, _)| name == "auditor"));
}

/// Each case: request, expected status, value the message must name.
#[tokio::test]
async fn invalid_changes_are_refused_with_4xx_naming_the_value() {
    let (server, policy) = admin_server().await;
    let before = std::fs::read_to_string(&policy).unwrap();
    let cases: [(&str, Value, u16, &str); 5] = [
        (
            "roles",
            json!({ "name": "auditor", "permissions": ["veiw"], "description": "Audit" }),
            400,
            "veiw",
        ),
        (
            "assignments",
            json!({ "user_id": "jane@example.com", "role": "devloper", "scopes": ["staging"] }),
            400,
            "devloper",
        ),
        (
            "assignments",
            json!({ "user_id": "jane@example.com", "role": "developer", "scopes": ["stagin"] }),
            400,
            "stagin",
        ),
        (
            "scopes",
            json!({ "name": "staging", "description": "again" }),
            409,
            "staging",
        ),
        (
            "roles",
            json!({ "name": "developer", "permissions": ["view"], "description": "again" }),
            409,
            "developer",
        ),
    ];
    for (endpoint, body, status, named) in cases {
        let response = server
            .post(&format!("/api/v1/authenticated/admin/{endpoint}"))
            .json(&body)
            .await;
        assert_eq!(response.status_code(), status, "{endpoint} {body}");
        let message = message(&response);
        assert!(message.contains(named), "{endpoint}: {message}");
    }
    assert_eq!(std::fs::read_to_string(&policy).unwrap(), before);
}

#[tokio::test]
async fn empty_fields_are_refused_with_400() {
    let (server, _policy) = admin_server().await;
    for (endpoint, body) in [
        ("scopes", json!({ "name": " ", "description": "d" })),
        ("scopes", json!({ "name": "qa", "description": "" })),
        (
            "roles",
            json!({ "name": "", "permissions": ["view"], "description": "d" }),
        ),
        (
            "roles",
            json!({ "name": "r", "permissions": ["view"], "description": " " }),
        ),
        (
            "roles",
            json!({ "name": "r", "permissions": [], "description": "d" }),
        ),
        (
            "assignments",
            json!({ "user_id": "", "role": "admin", "scopes": ["staging"] }),
        ),
        (
            "assignments",
            json!({ "user_id": "@nodot", "role": "admin", "scopes": ["staging"] }),
        ),
        (
            "assignments",
            json!({ "user_id": "u@example.com", "role": "", "scopes": ["staging"] }),
        ),
        (
            "assignments",
            json!({ "user_id": "u@example.com", "role": "admin", "scopes": [] }),
        ),
    ] {
        let response = server
            .post(&format!("/api/v1/authenticated/admin/{endpoint}"))
            .json(&body)
            .await;
        assert_eq!(response.status_code(), 400, "{endpoint} {body}");
    }
}

#[tokio::test]
async fn valid_assignment_is_accepted() {
    let (server, _policy) = admin_server().await;
    let response = server
        .post("/api/v1/authenticated/admin/assignments")
        .json(&json!({ "user_id": "jane@example.com", "role": "developer", "scopes": ["staging"] }))
        .await;
    assert_eq!(response.status_code(), 200, "{}", response.text());
    let body: Value = response.json();
    assert_eq!(body["success"], true);
}
