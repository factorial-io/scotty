use std::collections::HashMap;

use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::DateTime;
use http_body_util::BodyExt;
use scotty_core::{
    apps::{
        app_data::{AppData, AppSettings, AppStatus, AppTtl},
        shared_app_list::AppDataVec,
    },
    tasks::{
        running_app_context::RunningAppContext,
        task_details::{State, TaskDetails},
    },
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::api::secure_response::SecureJson;

// Helper functions for test data creation
fn create_app_settings_with_env_vars(env_vars: HashMap<String, String>) -> AppSettings {
    AppSettings {
        public_services: vec![],
        domain: "test.example.com".to_string(),
        time_to_live: AppTtl::Days(7),
        destroy_on_ttl: false,
        basic_auth: None,
        disallow_robots: true,
        environment: env_vars,
        ..Default::default()
    }
}

fn create_test_app_data(settings: AppSettings) -> AppData {
    AppData {
        name: "test-app".to_string(),
        settings: Some(settings),
        services: vec![],
        docker_compose_path: "/path/to/docker-compose.yml".to_string(),
        root_directory: "/path/to/app".to_string(),
        status: AppStatus::Running,
        last_checked: None,
    }
}

// Helper function to extract the JSON body from an Axum response
async fn extract_json_body<T: IntoResponse>(response: T) -> Value {
    let response = response.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body();
    let bytes = BodyExt::collect(body)
        .await
        .expect("Failed to collect body")
        .to_bytes();

    serde_json::from_slice(&bytes).expect("Failed to parse JSON body")
}

#[tokio::test]
async fn test_secure_json_masks_app_settings_env_vars() {
    // Create AppSettings with sensitive environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "secret-api-key-12345".to_string());
    env_vars.insert(
        "DATABASE_URL".to_string(),
        "postgres://user:password@localhost/db".to_string(),
    );
    env_vars.insert("NORMAL_VAR".to_string(), "not-sensitive".to_string());

    let settings = create_app_settings_with_env_vars(env_vars.clone());

    // Create regular Json response
    let regular_json = Json(settings.clone());
    let regular_json_body = extract_json_body(regular_json).await;

    // Create SecureJson response
    let secure_json = SecureJson(settings.clone());
    let secure_json_body = extract_json_body(secure_json).await;

    // Verify regular JSON contains unmasked env vars
    assert_eq!(
        regular_json_body["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_eq!(
        regular_json_body["environment"]["DATABASE_URL"],
        json!("postgres://user:password@localhost/db")
    );

    // Verify SecureJson response contains masked env vars
    assert_ne!(
        secure_json_body["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_ne!(
        secure_json_body["environment"]["DATABASE_URL"],
        json!("postgres://user:password@localhost/db")
    );

    // Verify non-sensitive vars are unchanged
    assert_eq!(
        secure_json_body["environment"]["NORMAL_VAR"],
        json!("not-sensitive")
    );

    // Verify that other fields are unchanged
    assert_eq!(secure_json_body["domain"], json!("test.example.com"));

    // Verify masking pattern follows expected format
    let masked_api_key = secure_json_body["environment"]["API_KEY"].as_str().unwrap();
    assert!(masked_api_key.starts_with("*****"));
    assert!(masked_api_key.ends_with("45"));
}

#[tokio::test]
async fn test_secure_json_masks_app_data_env_vars() {
    // Create AppData with sensitive environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "secret-api-key-12345".to_string());
    env_vars.insert("DB_PASSWORD".to_string(), "supersecretpassword".to_string());

    let settings = create_app_settings_with_env_vars(env_vars.clone());
    let app_data = create_test_app_data(settings);

    // Create regular Json response
    let regular_json = Json(app_data.clone());
    let regular_json_body = extract_json_body(regular_json).await;

    // Create SecureJson response
    let secure_json = SecureJson(app_data.clone());
    let secure_json_body = extract_json_body(secure_json).await;

    // Verify regular JSON contains unmasked sensitive values
    assert_eq!(
        regular_json_body["settings"]["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_eq!(
        regular_json_body["settings"]["environment"]["DB_PASSWORD"],
        json!("supersecretpassword")
    );

    // Verify SecureJson response contains masked sensitive values
    assert_ne!(
        secure_json_body["settings"]["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_ne!(
        secure_json_body["settings"]["environment"]["DB_PASSWORD"],
        json!("supersecretpassword")
    );

    // Verify that other fields are unchanged
    assert_eq!(secure_json_body["name"], json!("test-app"));
    assert_eq!(secure_json_body["status"], json!("Running"));

    // Verify masking pattern follows expected format
    let masked_password = secure_json_body["settings"]["environment"]["DB_PASSWORD"]
        .as_str()
        .unwrap();
    assert!(masked_password.starts_with("*****"));
    assert!(masked_password.ends_with("word"));
}

#[tokio::test]
async fn test_secure_json_masks_app_data_vec_env_vars() {
    // Create AppDataVec with sensitive environment variables
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "secret-api-key-12345".to_string());
    env_vars.insert("AUTH_TOKEN".to_string(), "bearer-token-secret".to_string());

    let settings = create_app_settings_with_env_vars(env_vars.clone());
    let app_data = create_test_app_data(settings);

    // Create an AppDataVec with our app
    let app_data_vec = AppDataVec {
        apps: vec![app_data.clone()],
    };

    // Create regular Json response
    let regular_json = Json(app_data_vec);
    let regular_json_body = extract_json_body(regular_json).await;

    // Create SecureJson response - create a new instance with the same app data
    let secure_json = SecureJson(AppDataVec {
        apps: vec![app_data.clone()],
    });
    let secure_json_body = extract_json_body(secure_json).await;

    // Verify regular JSON contains unmasked sensitive values
    assert_eq!(
        regular_json_body["apps"][0]["settings"]["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_eq!(
        regular_json_body["apps"][0]["settings"]["environment"]["AUTH_TOKEN"],
        json!("bearer-token-secret")
    );

    // Verify SecureJson response contains masked sensitive values
    assert_ne!(
        secure_json_body["apps"][0]["settings"]["environment"]["API_KEY"],
        json!("secret-api-key-12345")
    );
    assert_ne!(
        secure_json_body["apps"][0]["settings"]["environment"]["AUTH_TOKEN"],
        json!("bearer-token-secret")
    );

    // Verify that the masked values follow our expected masking pattern
    let masked_api_key = secure_json_body["apps"][0]["settings"]["environment"]["API_KEY"]
        .as_str()
        .unwrap();
    assert!(masked_api_key.starts_with("*****")); // Should start with asterisks
    assert!(masked_api_key.ends_with("45")); // Should end with last 2-4 chars

    let masked_token = secure_json_body["apps"][0]["settings"]["environment"]["AUTH_TOKEN"]
        .as_str()
        .unwrap();
    assert!(masked_token.starts_with("*****")); // Should start with asterisks
    assert!(masked_token.ends_with("cret")); // Should end with last 2-4 chars
}

#[tokio::test]
async fn test_secure_json_masks_running_app_context_env_vars() {
    // Create environment variables with sensitive data
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "very-sensitive-key-456".to_string());
    env_vars.insert(
        "SECRET_TOKEN".to_string(),
        "super-secret-token-123".to_string(),
    );
    env_vars.insert("DEBUG_MODE".to_string(), "enabled".to_string()); // Not sensitive

    // Create settings with sensitive environment variables
    let settings = create_app_settings_with_env_vars(env_vars.clone());

    // Create app data with the settings
    let app_data = create_test_app_data(settings);

    // Create task details
    let task_details = TaskDetails {
        id: Uuid::new_v4(),
        command: "test-command".to_string(),
        state: State::Finished,
        start_time: DateTime::from(std::time::SystemTime::now()),
        finish_time: Some(DateTime::from(std::time::SystemTime::now())),
        last_exit_code: Some(0),
        app_name: Some("test-app".to_string()),
    };

    // Create RunningAppContext
    let running_context = RunningAppContext {
        task: task_details.clone(),
        app_data: app_data.clone(),
    };

    // Create regular Json response
    let regular_json = Json(running_context);
    let regular_json_body = extract_json_body(regular_json).await;

    // Create SecureJson response with a new instance
    let secure_json = SecureJson(RunningAppContext {
        task: task_details,
        app_data: app_data.clone(),
    });
    let secure_json_body = extract_json_body(secure_json).await;

    // Verify regular JSON contains unmasked sensitive values
    assert_eq!(
        regular_json_body["app_data"]["settings"]["environment"]["API_KEY"],
        json!("very-sensitive-key-456")
    );
    assert_eq!(
        regular_json_body["app_data"]["settings"]["environment"]["SECRET_TOKEN"],
        json!("super-secret-token-123")
    );

    // Verify SecureJson response contains masked sensitive values
    assert_ne!(
        secure_json_body["app_data"]["settings"]["environment"]["API_KEY"],
        json!("very-sensitive-key-456")
    );
    assert_ne!(
        secure_json_body["app_data"]["settings"]["environment"]["SECRET_TOKEN"],
        json!("super-secret-token-123")
    );

    // Verify non-sensitive values are unchanged
    assert_eq!(
        secure_json_body["app_data"]["settings"]["environment"]["DEBUG_MODE"],
        json!("enabled")
    );

    // Verify task details are unchanged (only checking the state as ID is dynamically generated)
    assert_eq!(secure_json_body["task"]["state"], json!("Finished"));
    assert_eq!(secure_json_body["task"]["command"], json!("test-command"));

    // Verify app name and other fields are unchanged
    assert_eq!(secure_json_body["app_data"]["name"], json!("test-app"));
    assert_eq!(secure_json_body["app_data"]["status"], json!("Running"));

    // Verify masking pattern follows expected format
    let masked_token = secure_json_body["app_data"]["settings"]["environment"]["SECRET_TOKEN"]
        .as_str()
        .unwrap();
    assert!(masked_token.starts_with("*****")); // Should start with asterisks
    assert!(masked_token.ends_with("123")); // Should end with last few chars
}
