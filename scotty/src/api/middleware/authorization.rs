use axum::{
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Extension,
};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::{
    api::{basic_auth::CurrentUser, error::AppError},
    app_state::SharedAppState,
    services::{
        authorization::{decision::Target, Permission},
        AuthorizationService,
    },
};

/// Authorization context added to request extensions
#[derive(Clone, Debug)]
pub struct AuthorizationContext {
    pub user: CurrentUser,
    #[allow(dead_code)] // Used for future permission caching
    pub effective_permissions: HashMap<String, Vec<String>>,
}

impl AuthorizationContext {
    // Removed unused can_access_app method
}

/// Middleware that adds authorization context to requests
pub async fn authorization_middleware(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    mut req: Request,
    next: Next,
) -> Response {
    let auth_service = &state.auth_service;

    // Check if authorization has any assignments configured
    if !auth_service.is_enabled().await {
        warn!("Authorization is not properly configured - no assignments found. Denying request for security.");
        return AppError::AuthorizationNotConfigured.into_response();
    }

    let user_id = AuthorizationService::get_user_id_for_authorization(&user);

    // Get user's effective permissions for debugging
    let effective_permissions = auth_service.get_user_permissions(&user_id).await;

    info!(
        "User {} has permissions: {:?}",
        user_id, effective_permissions
    );

    // Add authorization context to request
    let auth_context = AuthorizationContext {
        user: user.clone(),
        effective_permissions,
    };

    req.extensions_mut().insert(auth_context);

    next.run(req).await
}

/// Future type for the permission middleware
type PermissionFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>;

/// Middleware factory that creates permission-checking middleware for app-specific or global actions
pub fn require_permission(
    permission: Permission,
) -> impl Fn(State<SharedAppState>, Request, Next) -> PermissionFuture + Clone {
    move |State(state): State<SharedAppState>, req: Request, next: Next| {
        Box::pin(async move {
            // Get authorization context
            let auth_context: &AuthorizationContext = req.extensions().get().ok_or_else(|| {
                warn!("Authorization context not found in request");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let user_id = AuthorizationService::get_user_id_for_authorization(&auth_context.user);

            // Admin permissions are global. Routes without an app in the path
            // (e.g. pending custom actions across all apps) need the
            // permission in at least one of the user's scopes.
            let is_global_permission =
                matches!(permission, Permission::AdminRead | Permission::AdminWrite);
            let app_name = (!is_global_permission)
                .then(|| extract_app_name_from_request(&req))
                .flatten();
            let target = app_name.as_deref().map_or(Target::Global, Target::App);

            if let Err(denied) = state
                .auth_service
                .require(&user_id, permission.into(), target)
            {
                // A JSON body, so clients can show why the action was refused.
                return Ok(denied.into_response());
            }

            info!(
                "Access granted: user {} has {} permission",
                auth_context.user.email,
                permission.as_str()
            );

            Ok(next.run(req).await)
        })
    }
}

/// Extract app name from request using MatchedPath when available
/// Falls back to path parsing for backwards compatibility
fn extract_app_name_from_request(req: &Request) -> Option<String> {
    // First, try to use MatchedPath for more robust extraction
    if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        let template = matched_path.as_str();
        let actual_path = req.uri().path();

        // Check if template contains app parameter placeholders
        if template.contains("{app_id}") || template.contains("{app_name}") {
            return extract_app_name_from_template(template, actual_path);
        }
    }

    // Fallback to the original path parsing for backwards compatibility
    extract_app_name_from_path(req.uri().path())
}

/// Extract app name using route template matching by finding parameter position
fn extract_app_name_from_template(template: &str, actual_path: &str) -> Option<String> {
    let template_parts: Vec<&str> = template.split('/').collect();
    let path_parts: Vec<&str> = actual_path.split('/').collect();

    // Find the position of {app_id} or {app_name} in the template
    for (i, part) in template_parts.iter().enumerate() {
        if *part == "{app_id}" || *part == "{app_name}" {
            return path_parts.get(i).map(|s| s.to_string());
        }
    }

    None
}

/// Legacy path parsing function (kept for backwards compatibility)
fn extract_app_name_from_path(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    // Look for patterns like /api/v1/authenticated/apps/{action}/{app_name}
    if parts.len() >= 6
        && parts[0] == "api"
        && parts[1] == "v1"
        && parts[2] == "authenticated"
        && parts[3] == "apps"
    {
        return Some(parts[5].to_string());
    }

    // Look for patterns like /apps/{action}/{app_name} (legacy)
    if parts.len() >= 3 && parts[0] == "apps" {
        return Some(parts[2].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_app_name_from_path() {
        // Test new API v1 paths
        assert_eq!(
            extract_app_name_from_path("/api/v1/authenticated/apps/info/my-app"),
            Some("my-app".to_string())
        );

        assert_eq!(
            extract_app_name_from_path("/api/v1/authenticated/apps/info/cd-with-db"),
            Some("cd-with-db".to_string())
        );

        // Test legacy paths
        assert_eq!(
            extract_app_name_from_path("/apps/info/my-app"),
            Some("my-app".to_string())
        );

        assert_eq!(
            extract_app_name_from_path("/apps/shell/test-app"),
            Some("test-app".to_string())
        );

        assert_eq!(
            extract_app_name_from_path("/apps/run/my-complex-app-name"),
            Some("my-complex-app-name".to_string())
        );

        assert_eq!(extract_app_name_from_path("/health"), None);

        // Cross-app admin routes carry no app name in the path. `require_permission`
        // must fall back to a global permission check for these, otherwise the
        // app-specific permission (e.g. ActionApprove) could never be satisfied and
        // the endpoint would always return 403.
        assert_eq!(
            extract_app_name_from_path("/api/v1/authenticated/admin/actions/pending"),
            None
        );
    }

    #[test]
    fn test_extract_app_name_from_template() {
        // Test {app_id} pattern
        assert_eq!(
            extract_app_name_from_template(
                "/api/v1/authenticated/apps/info/{app_id}",
                "/api/v1/authenticated/apps/info/my-app"
            ),
            Some("my-app".to_string())
        );

        // Test {app_name} pattern
        assert_eq!(
            extract_app_name_from_template(
                "/api/v1/authenticated/apps/{app_name}/actions",
                "/api/v1/authenticated/apps/test-app/actions"
            ),
            Some("test-app".to_string())
        );

        // Test complex app names
        assert_eq!(
            extract_app_name_from_template(
                "/api/v1/authenticated/apps/run/{app_id}",
                "/api/v1/authenticated/apps/run/my-complex-app-name"
            ),
            Some("my-complex-app-name".to_string())
        );

        // Test template without app parameter
        assert_eq!(
            extract_app_name_from_template(
                "/api/v1/authenticated/apps/list",
                "/api/v1/authenticated/apps/list"
            ),
            None
        );
    }
}
