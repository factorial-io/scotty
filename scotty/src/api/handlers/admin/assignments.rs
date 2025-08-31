use crate::api::basic_auth::CurrentUser;
use crate::{
    api::error::AppError, app_state::SharedAppState, 
    services::authorization::types::Assignment,
};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AssignmentInfo {
    pub user_id: String,
    pub assignments: Vec<Assignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AssignmentsListResponse {
    pub assignments: Vec<AssignmentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateAssignmentRequest {
    pub user_id: String,
    pub role: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct CreateAssignmentResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RemoveAssignmentRequest {
    pub user_id: String,
    pub role: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct RemoveAssignmentResponse {
    pub success: bool,
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/authenticated/admin/assignments",
    responses(
        (status = 200, response = inline(AssignmentsListResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminRead required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn list_assignments_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!("Admin listing assignments for user: {}", user.email);

    // Get all assignments from authorization service
    let assignments: HashMap<String, Vec<Assignment>> = auth_service.list_assignments().await;

    let assignments_info: Vec<AssignmentInfo> = assignments
        .into_iter()
        .map(|(user_id, assignments)| AssignmentInfo {
            user_id,
            assignments,
        })
        .collect();

    let response = AssignmentsListResponse {
        assignments: assignments_info,
    };

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/api/v1/authenticated/admin/assignments",
    request_body = CreateAssignmentRequest,
    responses(
        (status = 200, response = inline(CreateAssignmentResponse)),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminWrite required"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_assignment_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<CreateAssignmentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let auth_service = &state.auth_service;

    info!(
        "Admin creating assignment for '{}' by user: {}",
        request.user_id, user.email
    );

    // Validate input
    if request.user_id.trim().is_empty() {
        return Ok(Json(CreateAssignmentResponse {
            success: false,
            message: "User ID cannot be empty".to_string(),
        }));
    }

    if request.role.trim().is_empty() {
        return Ok(Json(CreateAssignmentResponse {
            success: false,
            message: "Role cannot be empty".to_string(),
        }));
    }

    if request.scopes.is_empty() {
        return Ok(Json(CreateAssignmentResponse {
            success: false,
            message: "At least one scope must be specified".to_string(),
        }));
    }

    // Validate that role exists
    let existing_roles = auth_service.list_roles().await;
    if !existing_roles.iter().any(|(name, _)| name == &request.role) {
        return Ok(Json(CreateAssignmentResponse {
            success: false,
            message: format!("Role '{}' does not exist", request.role),
        }));
    }

    // Validate that scopes exist (unless wildcard)
    let existing_scopes = auth_service.list_scopes().await;
    for scope in &request.scopes {
        if scope != "*" && !existing_scopes.iter().any(|(name, _)| name == scope) {
            return Ok(Json(CreateAssignmentResponse {
                success: false,
                message: format!("Scope '{}' does not exist", scope),
            }));
        }
    }

    // Create the assignment
    match auth_service
        .assign_user_role(&request.user_id, &request.role, request.scopes)
        .await
    {
        Ok(_) => {
            info!(
                "Successfully created assignment for user '{}' with role '{}'",
                request.user_id, request.role
            );
            Ok(Json(CreateAssignmentResponse {
                success: true,
                message: format!(
                    "Assignment created successfully for user '{}'",
                    request.user_id
                ),
            }))
        }
        Err(e) => {
            tracing::error!(
                "Failed to create assignment for user '{}': {}",
                request.user_id,
                e
            );
            Ok(Json(CreateAssignmentResponse {
                success: false,
                message: format!("Failed to create assignment: {}", e),
            }))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/authenticated/admin/assignments",
    request_body = RemoveAssignmentRequest,
    responses(
        (status = 200, response = inline(RemoveAssignmentResponse)),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 403, description = "Insufficient permissions - AdminWrite required"),
        (status = 404, description = "Assignment not found"),
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn remove_assignment_handler(
    State(_state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(request): Json<RemoveAssignmentRequest>,
) -> Result<impl IntoResponse, AppError> {
    info!(
        "Admin removing assignment for '{}' by user: {}",
        request.user_id, user.email
    );

    // For now, return a placeholder response
    // TODO: Implement remove_assignment method in AuthorizationService
    Ok(Json(RemoveAssignmentResponse {
        success: false,
        message: "Assignment removal not yet implemented".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use crate::services::authorization::AuthorizationService;

    #[tokio::test]
    async fn test_list_assignments_with_fallback_service() {
        let auth_service =
            AuthorizationService::create_fallback_service(Some("test-token".to_string())).await;

        let assignments = auth_service.list_assignments().await;

        // Fallback service should have at least one assignment for the token
        assert!(!assignments.is_empty());
    }

    #[tokio::test]
    async fn test_create_assignment_validation() {
        let auth_service =
            AuthorizationService::create_fallback_service(Some("test-token".to_string())).await;

        // Test creating a valid assignment
        let result = auth_service
            .assign_user_role("test-user", "admin", vec!["default".to_string()])
            .await;
        assert!(result.is_ok());

        // Verify assignment was created
        let assignments = auth_service.list_assignments().await;
        assert!(assignments.contains_key("test-user"));
    }
}