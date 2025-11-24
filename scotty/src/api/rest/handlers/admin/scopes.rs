use crate::api::basic_auth::CurrentUser;
use crate::{api::error::AppError, app_state::SharedAppState};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use scotty_core::admin::{CreateScopeRequest, CreateScopeResponse, ScopeInfo, ScopesListResponse};
use tracing::info;

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/scopes",
    responses(
        (status = 200, response = inline(ScopesListResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_scopes_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!("Admin listing scopes for user: {}", user.email);

    // Get all scopes from authorization service
    let scopes = auth_service.list_scopes().await;

    let scopes_info: Vec<ScopeInfo> = scopes
        .into_iter()
        .map(|(name, config)| ScopeInfo {
            name,
            description: config.description,
            created_at: config.created_at.to_rfc3339(),
        })
        .collect();

    let response = ScopesListResponse {
        scopes: scopes_info,
    };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/scopes",
    request_body = CreateScopeRequest,
    responses(
        (status = 200, response = inline(CreateScopeResponse)),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminWrite required"),
        (status = 409, description = "Scope already exists"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_scope_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<CreateScopeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!(
        "Admin creating scope '{}' for user: {}",
        request.name, user.email
    );

    // Validate input
    if request.name.trim().is_empty() {
        return Ok(Json(CreateScopeResponse {
            success: false,
            message: "Scope name cannot be empty".to_string(),
        }));
    }

    if request.description.trim().is_empty() {
        return Ok(Json(CreateScopeResponse {
            success: false,
            message: "Scope description cannot be empty".to_string(),
        }));
    }

    // Check if scope already exists
    let existing_scopes = auth_service.list_scopes().await;
    if existing_scopes
        .iter()
        .any(|(name, _)| name == &request.name)
    {
        return Ok(Json(CreateScopeResponse {
            success: false,
            message: format!("Scope '{}' already exists", request.name),
        }));
    }

    // Create the scope
    match auth_service
        .create_scope(&request.name, &request.description)
        .await
    {
        Ok(_) => {
            info!("Successfully created scope '{}'", request.name);
            Ok(Json(CreateScopeResponse {
                success: true,
                message: format!("Scope '{}' created successfully", request.name),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to create scope '{}': {}", request.name, e);
            Ok(Json(CreateScopeResponse {
                success: false,
                message: format!("Failed to create scope: {}", e),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::services::authorization::AuthorizationService;
    use tempfile::tempdir;

    async fn create_test_service() -> (AuthorizationService, tempfile::TempDir) {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let config_dir = temp_dir.path().to_str().unwrap();

        // Create model.conf
        let model_content = r#"[request_definition]
r = sub, app, act

[policy_definition]
p = sub, scope, act

[role_definition]
g = _, _
g2 = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && g2(r.app, p.scope) && r.act == p.act"#;

        std::fs::write(format!("{}/model.conf", config_dir), model_content)
            .expect("Failed to write model.conf");

        // Create empty policy.yaml
        let policy_content = r#"scopes:
  default:
    description: "Default scope"
    created_at: "2023-01-01T00:00:00Z"
roles:
  admin:
    description: "Admin role"
    permissions: ["*"]
assignments: {}
apps: {}"#;

        std::fs::write(format!("{}/policy.yaml", config_dir), policy_content)
            .expect("Failed to write policy.yaml");

        let service = AuthorizationService::new(config_dir)
            .await
            .expect("Failed to create service");
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_list_scopes_with_test_service() {
        let (auth_service, _temp_dir) = create_test_service().await;

        let scopes = auth_service.list_scopes().await;

        // Test service should have at least the default scope
        assert!(!scopes.is_empty());
        assert!(scopes.iter().any(|(name, _)| name == "default"));
    }

    #[tokio::test]
    async fn test_create_scope_validation() {
        let (auth_service, _temp_dir) = create_test_service().await;

        // Test creating a valid scope
        let result = auth_service.create_scope("test-scope", "Test scope").await;
        assert!(result.is_ok());

        // Verify scope was created
        let scopes = auth_service.list_scopes().await;
        assert!(scopes.iter().any(|(name, _)| name == "test-scope"));
    }
}
