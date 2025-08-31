use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
    Extension,
};
use std::collections::HashMap;
use tracing::{info, warn};

use crate::{
    api::basic_auth::CurrentUser,
    app_state::SharedAppState,
    services::{authorization::Permission, AuthorizationService},
};

/// Authorization context added to request extensions
#[derive(Clone, Debug)]
pub struct AuthorizationContext {
    pub user: CurrentUser,
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
) -> Result<Response, StatusCode> {
    let auth_service = &state.auth_service;

    // Check if authorization has any assignments configured
    if !auth_service.is_enabled().await {
        info!("Authorization has no assignments configured, allowing request");
        return Ok(next.run(req).await);
    }

    let user_id = AuthorizationService::format_user_id(&user.email, user.access_token.as_deref());

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

    Ok(next.run(req).await)
}

/// Future type for the permission middleware
type PermissionFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>>;

/// Middleware factory that creates permission-checking middleware for specific actions
pub fn require_permission(
    permission: Permission,
) -> impl Fn(State<SharedAppState>, Request, Next) -> PermissionFuture + Clone {
    move |State(state): State<SharedAppState>, req: Request, next: Next| {
        Box::pin(async move {
            // Extract app name from path
            let app_name = extract_app_name_from_path(req.uri().path());

            if app_name.is_none() {
                warn!("Could not extract app name from path: {}", req.uri().path());
                return Err(StatusCode::BAD_REQUEST);
            }

            let app_name = app_name.unwrap();

            // Get authorization context
            let auth_context: &AuthorizationContext = req.extensions().get().ok_or_else(|| {
                warn!("Authorization context not found in request");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let auth_service = &state.auth_service;

            // Check permission
            let user_id = AuthorizationService::format_user_id(
                &auth_context.user.email,
                auth_context.user.access_token.as_deref(),
            );
            let allowed = auth_service
                .check_permission(&user_id, &app_name, &permission)
                .await;

            if !allowed {
                warn!(
                    "Access denied: user {} cannot {} on app {}",
                    auth_context.user.email,
                    permission.as_str(),
                    app_name
                );
                return Err(StatusCode::FORBIDDEN);
            }

            info!(
                "Access granted: user {} can {} on app {}",
                auth_context.user.email,
                permission.as_str(),
                app_name
            );

            Ok(next.run(req).await)
        })
    }
}

/// Extract app name from request path
/// Supports patterns like /api/v1/authenticated/apps/info/{app_name}, /apps/shell/{app_name}, etc.
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
    }
}
