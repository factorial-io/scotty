use crate::api::basic_auth::CurrentUser;
use crate::{
    api::error::AppError,
    app_state::SharedAppState,
    services::authorization::{AuthorizationService, Permission},
};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TestPermissionRequest {
    pub user_id: Option<String>, // If None, test current user
    pub app_name: String,
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct TestPermissionResponse {
    pub user_id: String,
    pub app_name: String,
    pub permission: String,
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct UserPermissionsResponse {
    pub user_id: String,
    pub permissions: std::collections::HashMap<String, Vec<String>>,
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/permissions/test",
    request_body = TestPermissionRequest,
    responses(
        (status = 200, response = inline(TestPermissionResponse)),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn test_permission_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<TestPermissionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    // Determine which user to test
    let test_user_id = match request.user_id {
        Some(user_id) => user_id,
        None => AuthorizationService::format_user_id(&user.email, user.access_token.as_deref()),
    };

    info!(
        "Admin testing permission '{}' for user '{}' on app '{}' by admin: {}",
        request.permission, test_user_id, request.app_name, user.email
    );

    // Parse permission
    let permission = match Permission::from_str(&request.permission) {
        Some(perm) => perm,
        None => {
            return Ok(Json(TestPermissionResponse {
                user_id: test_user_id,
                app_name: request.app_name.clone(),
                permission: request.permission.clone(),
                allowed: false,
                reason: Some(format!("Invalid permission: '{}'", request.permission)),
            }));
        }
    };

    // Test the permission
    let allowed = auth_service
        .check_permission(&test_user_id, &request.app_name, &permission)
        .await;

    let response = TestPermissionResponse {
        user_id: test_user_id,
        app_name: request.app_name,
        permission: request.permission,
        allowed,
        reason: if allowed {
            None
        } else {
            Some("Permission denied".to_string())
        },
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/permissions/user/{user_id}",
    responses(
        (status = 200, response = inline(UserPermissionsResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
        (status = 404, description = "User not found"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_user_permissions_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!(
        "Admin getting permissions for user '{}' by admin: {}",
        user_id, user.email
    );

    // Get user's effective permissions
    let permissions = auth_service.get_user_permissions(&user_id).await;

    let response = UserPermissionsResponse {
        user_id,
        permissions,
    };

    Ok(Json(response))
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AvailablePermissionsResponse {
    pub permissions: Vec<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/permissions",
    responses(
        (status = 200, response = inline(AvailablePermissionsResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_available_permissions_handler(
    State(_state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Admin listing available permissions by user: {}",
        user.email
    );

    let permissions: Vec<String> = Permission::all()
        .into_iter()
        .map(|p| p.as_str().to_string())
        .collect();

    let response = AvailablePermissionsResponse { permissions };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::AuthorizationService;

    #[tokio::test]
    async fn test_permission_testing() {
        let auth_service =
            AuthorizationService::create_fallback_service(Some("test-token".to_string())).await;

        let user_id = AuthorizationService::format_user_id("", Some("test-token"));

        // Test with a permission the token should have (admin gets all permissions)
        let allowed = auth_service
            .check_permission(&user_id, "test-app", &Permission::View)
            .await;
        assert!(allowed);

        // Test with an invalid app (should still work for admin)
        let allowed = auth_service
            .check_permission(&user_id, "nonexistent-app", &Permission::View)
            .await;
        assert!(allowed); // Admin has wildcard permissions
    }

    #[tokio::test]
    async fn test_list_available_permissions() {
        let permissions = Permission::all();
        assert!(permissions.contains(&Permission::View));
        assert!(permissions.contains(&Permission::AdminRead));
        assert!(permissions.contains(&Permission::AdminWrite));
    }

    #[tokio::test]
    async fn test_get_user_permissions() {
        let auth_service =
            AuthorizationService::create_fallback_service(Some("test-token".to_string())).await;

        let user_id = AuthorizationService::format_user_id("", Some("test-token"));
        let permissions = auth_service.get_user_permissions(&user_id).await;

        // Fallback service gives admin permissions, so should have default scope with all permissions
        assert!(!permissions.is_empty());
        assert!(permissions.contains_key("default"));
    }
}
