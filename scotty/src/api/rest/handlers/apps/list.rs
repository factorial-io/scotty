use axum::{debug_handler, extract::State, response::IntoResponse, Extension};
use scotty_core::apps::shared_app_list::AppDataVec;

use crate::{
    api::basic_auth::CurrentUser,
    api::error::AppError,
    api::secure_response::SecureJson,
    app_state::SharedAppState,
    services::{authorization::Permission, AuthorizationService},
};
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/apps/list",
    responses(
    (status = 200, response = inline(AppDataVec)),
    (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
#[debug_handler]
pub async fn list_apps_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    let all_apps = state.apps.get_apps().await;

    tracing::info!("Total apps discovered: {}", all_apps.apps.len());
    for app in &all_apps.apps {
        if let Some(settings) = &app.settings {
            tracing::info!(
                "Discovered app: {} (groups: {:?})",
                app.name,
                settings.scopes
            );
        } else {
            tracing::info!("Discovered app: {} (no settings)", app.name);
        }
    }

    let auth_service = &state.auth_service;

    // If authorization is enabled but no assignments exist, return all apps
    if !auth_service.is_enabled().await {
        return Ok(SecureJson(all_apps));
    }

    // Filter apps based on user's view permissions
    let user_id = AuthorizationService::get_user_id_for_authorization(&user);
    tracing::info!(
        "Filtering apps for user_id: {}, email: {}, token: {:?}",
        user_id,
        user.email,
        user.access_token
    );

    let mut filtered_apps = Vec::new();

    for app in all_apps.apps {
        let has_permission = auth_service
            .check_permission(&user_id, &app.name, &Permission::View)
            .await;
        tracing::info!(
            "App '{}' permission check for user '{}': {}",
            app.name,
            user_id,
            has_permission
        );

        if has_permission {
            filtered_apps.push(app);
        }
    }

    Ok(SecureJson(AppDataVec {
        apps: filtered_apps,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app_state::AppState, services::authorization::AuthorizationService,
        settings::config::Settings, stop_flag,
    };
    use axum::body::to_bytes;
    use scotty_core::apps::{
        app_data::{AppData, AppSettings, AppStatus},
        shared_app_list::{AppDataVec as CoreAppDataVec, SharedAppList},
    };
    use std::sync::Arc;
    use tempfile::tempdir;
    use tempfile::TempDir;

    /// Helper function to create test WebSocket messenger
    fn create_test_websocket_messenger() -> crate::api::websocket::WebSocketMessenger {
        use crate::api::websocket::WebSocketMessenger;
        let clients = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
        WebSocketMessenger::new(clients)
    }

    async fn create_test_auth_service() -> (Arc<AuthorizationService>, TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_dir = temp_dir.path().to_str().unwrap();

        // Create model.conf
        let model_content = r#"[request_definition]
r = sub, app, act

[policy_definition]
p = sub, group, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && g2(r.app, p.group) && r.act == p.act
"#;
        tokio::fs::write(format!("{}/model.conf", config_dir), model_content)
            .await
            .unwrap();

        let service = AuthorizationService::new(config_dir).await.unwrap();

        // Create groups
        service
            .create_scope("frontend", "Frontend applications")
            .await
            .unwrap();
        service
            .create_scope("backend", "Backend services")
            .await
            .unwrap();
        service
            .create_scope("staging", "Staging environment")
            .await
            .unwrap();

        // Use the existing "developer" role from default config
        // Set up user permissions
        service
            .assign_user_role(
                "frontend-dev@example.com",
                "developer",
                vec!["frontend".to_string()],
            )
            .await
            .unwrap();

        service
            .assign_user_role(
                "backend-dev@example.com",
                "developer",
                vec!["backend".to_string()],
            )
            .await
            .unwrap();

        service
            .assign_user_role(
                "full-stack-dev@example.com",
                "developer",
                vec!["frontend".to_string(), "backend".to_string()],
            )
            .await
            .unwrap();

        (Arc::new(service), temp_dir)
    }

    async fn create_test_app_state() -> (SharedAppState, TempDir) {
        let (auth_service, temp_dir) = create_test_auth_service().await;

        // Create test settings using default implementation
        let settings = Settings::default();
        let shared_app_list = SharedAppList::new();

        // Create mock apps with different group memberships
        let frontend_settings = AppSettings {
            scopes: vec!["frontend".to_string()],
            ..Default::default()
        };

        let backend_settings = AppSettings {
            scopes: vec!["backend".to_string()],
            ..Default::default()
        };

        let fullstack_settings = AppSettings {
            scopes: vec!["frontend".to_string(), "backend".to_string()],
            ..Default::default()
        };

        let staging_settings = AppSettings {
            scopes: vec!["staging".to_string()],
            ..Default::default()
        };

        let frontend_app = AppData {
            name: "frontend-app".to_string(),
            root_directory: "/test/frontend-app".to_string(),
            docker_compose_path: "/test/frontend-app/docker-compose.yml".to_string(),
            status: AppStatus::Running,
            services: Vec::new(),
            settings: Some(frontend_settings),
            last_checked: None,
        };

        let backend_app = AppData {
            name: "backend-app".to_string(),
            root_directory: "/test/backend-app".to_string(),
            docker_compose_path: "/test/backend-app/docker-compose.yml".to_string(),
            status: AppStatus::Running,
            services: Vec::new(),
            settings: Some(backend_settings),
            last_checked: None,
        };

        let fullstack_app = AppData {
            name: "fullstack-app".to_string(),
            root_directory: "/test/fullstack-app".to_string(),
            docker_compose_path: "/test/fullstack-app/docker-compose.yml".to_string(),
            status: AppStatus::Running,
            services: Vec::new(),
            settings: Some(fullstack_settings),
            last_checked: None,
        };

        let staging_app = AppData {
            name: "staging-app".to_string(),
            root_directory: "/test/staging-app".to_string(),
            docker_compose_path: "/test/staging-app/docker-compose.yml".to_string(),
            status: AppStatus::Running,
            services: Vec::new(),
            settings: Some(staging_settings),
            last_checked: None,
        };

        // Add apps to shared list
        let apps_vec = CoreAppDataVec {
            apps: vec![frontend_app, backend_app, fullstack_app, staging_app],
        };
        shared_app_list.set_apps(&apps_vec).await.unwrap();

        // Sync app groups to authorization service (simulating app discovery)
        for app in shared_app_list.get_apps().await.apps {
            if let Some(settings) = &app.settings {
                auth_service
                    .set_app_scopes(&app.name, settings.scopes.clone())
                    .await
                    .unwrap();
            }
        }

        let docker = bollard::Docker::connect_with_local_defaults().unwrap();
        let app_state = Arc::new(AppState {
            settings,
            stop_flag: stop_flag::StopFlag::new(),
            messenger: create_test_websocket_messenger(),
            apps: shared_app_list,
            docker: docker.clone(),
            task_manager: crate::tasks::manager::TaskManager::new(create_test_websocket_messenger()),
            oauth_state: None,
            auth_service,
            logs_service: crate::docker::services::logs::LogStreamingService::new(docker),
        });

        (app_state, temp_dir)
    }

    #[tokio::test]
    async fn test_list_apps_filtered_by_user_groups() {
        let (app_state, _temp_dir) = create_test_app_state().await;

        // Test frontend developer - should only see frontend and fullstack apps
        let frontend_user = CurrentUser {
            email: "frontend-dev@example.com".to_string(),
            name: "Frontend Dev".to_string(),
            picture: None,
            access_token: None,
        };

        let result = list_apps_handler(State(app_state.clone()), Extension(frontend_user))
            .await
            .unwrap();

        let response_body = to_bytes(result.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        let apps: AppDataVec = serde_json::from_slice(&response_body).unwrap();

        // Should see 2 apps: frontend-app and fullstack-app
        assert_eq!(apps.apps.len(), 2);
        let app_names: Vec<&str> = apps.apps.iter().map(|a| a.name.as_str()).collect();
        assert!(app_names.contains(&"frontend-app"));
        assert!(app_names.contains(&"fullstack-app"));
        assert!(!app_names.contains(&"backend-app"));
        assert!(!app_names.contains(&"staging-app"));
    }

    #[tokio::test]
    async fn test_list_apps_backend_user() {
        let (app_state, _temp_dir) = create_test_app_state().await;

        // Test backend developer - should only see backend and fullstack apps
        let backend_user = CurrentUser {
            email: "backend-dev@example.com".to_string(),
            name: "Backend Dev".to_string(),
            picture: None,
            access_token: None,
        };

        let result = list_apps_handler(State(app_state.clone()), Extension(backend_user))
            .await
            .unwrap();

        let response_body = to_bytes(result.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        let apps: AppDataVec = serde_json::from_slice(&response_body).unwrap();

        // Should see 2 apps: backend-app and fullstack-app
        assert_eq!(apps.apps.len(), 2);
        let app_names: Vec<&str> = apps.apps.iter().map(|a| a.name.as_str()).collect();
        assert!(app_names.contains(&"backend-app"));
        assert!(app_names.contains(&"fullstack-app"));
        assert!(!app_names.contains(&"frontend-app"));
        assert!(!app_names.contains(&"staging-app"));
    }

    #[tokio::test]
    async fn test_list_apps_full_stack_user() {
        let (app_state, _temp_dir) = create_test_app_state().await;

        // Test full-stack developer - should see frontend, backend, and fullstack apps
        let fullstack_user = CurrentUser {
            email: "full-stack-dev@example.com".to_string(),
            name: "Full Stack Dev".to_string(),
            picture: None,
            access_token: None,
        };

        let result = list_apps_handler(State(app_state.clone()), Extension(fullstack_user))
            .await
            .unwrap();

        let response_body = to_bytes(result.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        let apps: AppDataVec = serde_json::from_slice(&response_body).unwrap();

        // Should see 3 apps: frontend-app, backend-app, and fullstack-app
        assert_eq!(apps.apps.len(), 3);
        let app_names: Vec<&str> = apps.apps.iter().map(|a| a.name.as_str()).collect();
        assert!(app_names.contains(&"frontend-app"));
        assert!(app_names.contains(&"backend-app"));
        assert!(app_names.contains(&"fullstack-app"));
        assert!(!app_names.contains(&"staging-app"));

        println!("✅ Full-stack user sees correct apps: {:?}", app_names);
    }

    #[tokio::test]
    async fn test_list_apps_no_permissions() {
        let (app_state, _temp_dir) = create_test_app_state().await;

        // Test user with no permissions - should see no apps
        let no_permissions_user = CurrentUser {
            email: "no-access@example.com".to_string(),
            name: "No Access User".to_string(),
            picture: None,
            access_token: None,
        };

        let result = list_apps_handler(State(app_state.clone()), Extension(no_permissions_user))
            .await
            .unwrap();

        let response_body = to_bytes(result.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        let apps: AppDataVec = serde_json::from_slice(&response_body).unwrap();

        // Should see 0 apps
        assert_eq!(apps.apps.len(), 0);

        println!("✅ User with no permissions sees no apps");
    }
}
