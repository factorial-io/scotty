use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::error::AppError, app_state::SharedAppState, docker::services::shell::ShellService,
};
use scotty_core::utils::slugify::slugify;

#[derive(Deserialize, ToSchema)]
pub struct CreateShellRequest {
    /// Shell command to execute (optional, uses default shell if not provided)
    pub shell_command: Option<String>,
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct CreateShellResponse {
    pub session_id: Uuid,
    pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ShellInputRequest {
    /// Input data to send to the shell
    pub input: String,
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct ShellInputResponse {
    pub message: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ResizeTtyRequest {
    /// Terminal width in characters
    pub width: u16,
    /// Terminal height in characters
    pub height: u16,
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct ResizeTtyResponse {
    pub message: String,
}

#[derive(Serialize, ToSchema, utoipa::ToResponse)]
pub struct TerminateShellResponse {
    pub message: String,
}

/// Create a new shell session for a service
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/apps/{app_id}/services/{service_name}/shell",
    params(
        ("app_id" = String, Path, description = "Application ID"),
        ("service_name" = String, Path, description = "Service name to create shell session for")
    ),
    request_body = CreateShellRequest,
    responses(
        (status = 200, response = inline(CreateShellResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "App or service not found"),
        (status = 409, description = "Service has no container ID"),
        (status = 429, description = "Maximum sessions limit reached")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn create_shell_handler(
    Path((app_id, service_name)): Path<(String, String)>,
    State(state): State<SharedAppState>,
    Json(request): Json<CreateShellRequest>,
) -> Result<impl IntoResponse, AppError> {
    let app_id = slugify(&app_id);
    let app_data = state
        .apps
        .get_app(&app_id)
        .await
        .ok_or_else(|| AppError::AppNotFound(app_id.clone()))?;

    let shell_service = ShellService::new(state.docker.clone(), state.settings.shell.clone());

    let session_id = shell_service
        .create_session(&state, &app_data, &service_name, request.shell_command)
        .await?;

    Ok(axum::Json(CreateShellResponse {
        session_id,
        message: format!("Created shell session for {}/{}", app_id, service_name),
    }))
}

/// Send input to a shell session
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/shell/sessions/{session_id}/input",
    params(
        ("session_id" = Uuid, Path, description = "Shell session ID")
    ),
    request_body = ShellInputRequest,
    responses(
        (status = 200, response = inline(ShellInputResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "Session not found")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn shell_input_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<SharedAppState>,
    Json(request): Json<ShellInputRequest>,
) -> Result<impl IntoResponse, AppError> {
    let shell_service = ShellService::new(state.docker.clone(), state.settings.shell.clone());

    shell_service.send_input(session_id, request.input).await?;

    Ok(axum::Json(ShellInputResponse {
        message: format!("Sent input to shell session {}", session_id),
    }))
}

/// Resize a shell session's TTY
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/shell/sessions/{session_id}/resize",
    params(
        ("session_id" = Uuid, Path, description = "Shell session ID")
    ),
    request_body = ResizeTtyRequest,
    responses(
        (status = 200, response = inline(ResizeTtyResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "Session not found")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn resize_tty_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<SharedAppState>,
    Json(request): Json<ResizeTtyRequest>,
) -> Result<impl IntoResponse, AppError> {
    let shell_service = ShellService::new(state.docker.clone(), state.settings.shell.clone());

    shell_service
        .resize_tty(session_id, request.width, request.height)
        .await?;

    Ok(axum::Json(ResizeTtyResponse {
        message: format!("Resized TTY for shell session {}", session_id),
    }))
}

/// Terminate a shell session
#[utoipa::path(
    delete,
    path = "/api/v1/authenticated/shell/sessions/{session_id}",
    params(
        ("session_id" = Uuid, Path, description = "Shell session ID")
    ),
    responses(
        (status = 200, response = inline(TerminateShellResponse)),
        (status = 401, description = "Access token is missing or invalid"),
        (status = 404, description = "Session not found")
    ),
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn terminate_shell_handler(
    Path(session_id): Path<Uuid>,
    State(state): State<SharedAppState>,
) -> Result<impl IntoResponse, AppError> {
    let shell_service = ShellService::new(state.docker.clone(), state.settings.shell.clone());

    shell_service.terminate_session(session_id).await?;

    Ok(axum::Json(TerminateShellResponse {
        message: format!("Terminated shell session {}", session_id),
    }))
}
