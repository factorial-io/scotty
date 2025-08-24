use crate::api::basic_auth::CurrentUser;
use crate::{
    api::error::AppError, app_state::SharedAppState, services::authorization::AuthorizationService,
};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct GroupInfo {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct UserGroupsResponse {
    pub groups: Vec<GroupInfo>,
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/groups/list",
    responses(
        (status = 200, response = inline(UserGroupsResponse)),
        (status = 401, description = "Access token is missing or invalid"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_user_groups_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    // Get user ID
    let user_id = AuthorizationService::format_user_id(&user.email, user.access_token.as_deref());

    debug!("Fetching groups for user: {}", user_id);

    // Get user's groups with permissions
    let user_groups = auth_service
        .get_user_groups_with_permissions(&user_id)
        .await;

    let response = UserGroupsResponse {
        groups: user_groups,
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use crate::services::authorization::AuthorizationService;

    #[tokio::test]
    async fn test_list_user_groups_with_fallback_token() {
        // Create a test authorization service with a token
        let test_token = "test-token-123";
        let auth_service =
            AuthorizationService::create_fallback_service(Some(test_token.to_string())).await;

        // Verify the token user has admin role for default group
        let user_id = AuthorizationService::format_user_id("", Some(test_token));
        let user_groups = auth_service
            .get_user_groups_with_permissions(&user_id)
            .await;

        // Should have one group (default) with admin permissions (*)
        assert_eq!(user_groups.len(), 1);
        assert_eq!(user_groups[0].name, "default");
        assert!(user_groups[0].permissions.contains(&"*".to_string()));
    }
}
