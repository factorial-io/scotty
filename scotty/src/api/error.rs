#![allow(dead_code)]

use crate::services::authorization::policy::PolicyError;
use axum::http::StatusCode;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use scotty_core::auth::OAuthError;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Error, Debug, utoipa::ToResponse, utoipa::ToSchema)]
pub enum AppError {
    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Not found")]
    NotFound,

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("Invalid input")]
    InvalidInput,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("File content could not be decoded!")]
    FileContentDecodingError,

    #[error(
        "Request body ({0}) exceeds the configured limit of {1}; \
         raise api.create_app_max_size (SCOTTY__API__CREATE_APP_MAX_SIZE) on the server"
    )]
    CreateAppPayloadTooLarge(String, String),

    #[error("File compression corrupted for {0}: {1}")]
    FileCompressionCorrupted(String, String),

    #[error("File {0} exceeds maximum decompressed size of {1} bytes")]
    FileDecompressedSizeExceeded(String, usize),

    #[error("Can't destroy app {0}: Scotty does not manage it (adopt it first)")]
    CantDestroyUnmanagedApp(String),

    #[error("Missing docker-compose file in the payload")]
    NoDockerComposeFile,

    #[error("Invalid docker-compose file")]
    InvalidDockerComposeFile,

    #[error("Service not found in docker compose file: {0}")]
    PublicServiceNotFound(String),

    #[error("Public ports for service {0} are not supported")]
    PublicPortsNotSupported(String),

    #[error("Environment variable {0} for variable substitution is missing")]
    EnvironmentVariablesNotSupported(String),

    #[error("Operation not supported for legacy/ unsupported app {0}")]
    OperationNotSupportedForLegacyApp(String),

    #[error("Private registry not found in settings: {0}")]
    RegistryNotFound(String),

    #[error("App blueprint not found in settings: {0}")]
    AppBlueprintNotFound(String),

    #[error("App blueprint mismatch: {0}")]
    AppBlueprintMismatch(String),

    #[error("App settings not found for app: {0}")]
    AppSettingsNotFound(String),

    #[error("App is not running: {0}")]
    AppNotRunning(String),

    #[error("{0}")]
    ActionNotFound(String),

    #[error("Action not executable: {0}")]
    ActionNotExecutable(String),

    #[error("Action already exists: {0}")]
    ActionAlreadyExists(String),

    #[error("App {app} is busy: {}; try again when it has finished", describe_operation(.operation))]
    OperationInProgress {
        app: String,
        operation: scotty_core::apps::app_data::AppOperation,
    },

    #[error("{0}")]
    ActionInvalidState(String),

    #[error("Found invalid notification service ids: {0}")]
    InvalidNotificationServiceIds(String),

    #[error("Can't create app from an existing .scotty.yml file")]
    CantCreateAppWithScottyYmlFile,

    #[error("Cant adopt app {0} with existing settings, app can already be controlled by scotty!")]
    CantAdoptAppWithExistingSettings(String),

    #[error("Middleware not allowed: {0}")]
    MiddlewareNotAllowed(String),

    #[error("OAuth error: {0}")]
    OAuthError(OAuthError),

    #[error("Scopes not found in authorization system: {0:?}")]
    ScopesNotFound(Vec<String>),

    #[error("Authorization system is not properly configured - no assignments found")]
    AuthorizationNotConfigured,

    #[error("Access denied: {0}")]
    ScopeAccessDenied(String),

    #[error("Log stream error: {0}")]
    LogStreamError(crate::docker::services::logs::LogStreamError),

    #[error("Shell service error: {0}")]
    ShellServiceError(crate::docker::services::shell::ShellServiceError),
}
/// e.g. `rebuild (task 6f1c…) running since 2026-11-12T10:03:11Z`.
fn describe_operation(operation: &scotty_core::apps::app_data::AppOperation) -> String {
    let task = operation
        .task_id
        .map(|id| format!(" (task {id})"))
        .unwrap_or_default();
    format!(
        "{}{task} running since {}",
        operation.operation,
        scotty_types::time::to_rfc3339_secs(&operation.started_at)
    )
}

impl AppError {
    fn get_error_msg(&self) -> (axum::http::StatusCode, String) {
        let status: axum::http::StatusCode = match self {
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InternalServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::AppNotFound(_) => StatusCode::NOT_FOUND,
            AppError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            AppError::AppSettingsNotFound(_) => StatusCode::NOT_FOUND,
            AppError::CantCreateAppWithScottyYmlFile => StatusCode::BAD_REQUEST,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::CreateAppPayloadTooLarge(_, _) => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::FileCompressionCorrupted(_, _) => StatusCode::BAD_REQUEST,
            AppError::FileDecompressedSizeExceeded(_, _) => StatusCode::BAD_REQUEST,
            AppError::CantAdoptAppWithExistingSettings(_) => StatusCode::BAD_REQUEST,
            AppError::MiddlewareNotAllowed(_) => StatusCode::BAD_REQUEST,
            AppError::AppNotRunning(_) => StatusCode::CONFLICT,
            AppError::ActionNotFound(_) => StatusCode::NOT_FOUND,
            AppError::ActionNotExecutable(_) => StatusCode::FORBIDDEN,
            AppError::ActionAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::OperationInProgress { .. } => StatusCode::CONFLICT,
            // The app's state doesn't allow the operation.
            AppError::CantDestroyUnmanagedApp(_)
            | AppError::OperationNotSupportedForLegacyApp(_) => StatusCode::CONFLICT,
            // The request itself is wrong; retrying it can't help.
            AppError::InvalidInput
            | AppError::FileContentDecodingError
            | AppError::NoDockerComposeFile
            | AppError::InvalidDockerComposeFile
            | AppError::PublicServiceNotFound(_)
            | AppError::PublicPortsNotSupported(_)
            | AppError::EnvironmentVariablesNotSupported(_)
            | AppError::RegistryNotFound(_)
            | AppError::AppBlueprintNotFound(_)
            | AppError::AppBlueprintMismatch(_)
            | AppError::InvalidNotificationServiceIds(_) => StatusCode::BAD_REQUEST,
            AppError::ActionInvalidState(_) => StatusCode::BAD_REQUEST,
            AppError::OAuthError(ref oauth_error) => oauth_error.clone().into(),
            AppError::ScopesNotFound(_) => StatusCode::BAD_REQUEST,
            AppError::AuthorizationNotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            AppError::ScopeAccessDenied(_) => StatusCode::FORBIDDEN,
            AppError::LogStreamError(ref e) => match e {
                crate::docker::services::logs::LogStreamError::ServiceNotFound { .. } => {
                    StatusCode::NOT_FOUND
                }
                crate::docker::services::logs::LogStreamError::NoContainerId { .. } => {
                    StatusCode::CONFLICT
                }
                crate::docker::services::logs::LogStreamError::StreamNotFound { .. } => {
                    StatusCode::NOT_FOUND
                }
                crate::docker::services::logs::LogStreamError::CommandSendFailed { .. } => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                crate::docker::services::logs::LogStreamError::DockerOperationFailed { .. } => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
            },
            AppError::ShellServiceError(ref e) => match e {
                crate::docker::services::shell::ShellServiceError::ServiceNotFound { .. } => {
                    StatusCode::NOT_FOUND
                }
                crate::docker::services::shell::ShellServiceError::NoContainerId { .. } => {
                    StatusCode::CONFLICT
                }
                crate::docker::services::shell::ShellServiceError::SessionNotFound { .. } => {
                    StatusCode::NOT_FOUND
                }
                crate::docker::services::shell::ShellServiceError::MaxSessionsPerApp { .. } => {
                    StatusCode::TOO_MANY_REQUESTS
                }
                crate::docker::services::shell::ShellServiceError::MaxSessionsGlobal { .. } => {
                    StatusCode::TOO_MANY_REQUESTS
                }
                crate::docker::services::shell::ShellServiceError::CommandSendFailed { .. } => {
                    StatusCode::INTERNAL_SERVER_ERROR
                }
                crate::docker::services::shell::ShellServiceError::DockerOperationFailed {
                    ..
                } => StatusCode::INTERNAL_SERVER_ERROR,
            },
        };

        (status, self.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        if let Some(app_error) = e.downcast_ref::<AppError>() {
            return app_error.clone();
        }
        if let Some(policy_error) = e.downcast_ref::<PolicyError>() {
            return policy_error.clone().into();
        }
        AppError::InternalServerError(e.to_string())
    }
}

/// A refused policy change: the request names a duplicate (409) or something
/// the policy does not define or accept (400).
impl From<PolicyError> for AppError {
    fn from(e: PolicyError) -> Self {
        match e {
            PolicyError::DuplicateScope(_) | PolicyError::DuplicateRole(_) => {
                AppError::Conflict(e.to_string())
            }
            _ => AppError::BadRequest(e.to_string()),
        }
    }
}

impl From<OAuthError> for AppError {
    fn from(oauth_error: OAuthError) -> Self {
        AppError::OAuthError(oauth_error)
    }
}

impl From<AppError> for scotty_core::auth::ErrorResponse {
    fn from(app_error: AppError) -> Self {
        match app_error {
            AppError::OAuthError(oauth_error) => oauth_error.into(),
            // For non-OAuth errors, create a generic error response
            _ => scotty_core::auth::ErrorResponse {
                error: "server_error".to_string(),
                error_description: Some(app_error.to_string()),
            },
        }
    }
}

impl From<crate::docker::services::logs::LogStreamError> for AppError {
    fn from(error: crate::docker::services::logs::LogStreamError) -> Self {
        AppError::LogStreamError(error)
    }
}

impl From<crate::docker::services::shell::ShellServiceError> for AppError {
    fn from(error: crate::docker::services::shell::ShellServiceError) -> Self {
        AppError::ShellServiceError(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            // For OAuth errors, return OAuth-compliant ErrorResponse
            AppError::OAuthError(oauth_error) => {
                let status: StatusCode = oauth_error.clone().into();
                let error_response: scotty_core::auth::ErrorResponse = oauth_error.clone().into();
                (status, Json(error_response)).into_response()
            }
            // For all other errors, return standard AppError format
            _ => {
                let (status, body) = self.get_error_msg();
                let mut body = serde_json::json!({ "error": true, "message": body });
                // The running operation, so a client can wait for it.
                if let AppError::OperationInProgress { operation, .. } = &self {
                    body["operation"] = serde_json::json!(operation);
                }
                (status, Json(body)).into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn operation_in_progress_is_409_naming_the_holder() {
        let task_id = Uuid::new_v4();
        let started_at = jiff::Timestamp::now();
        let response = AppError::OperationInProgress {
            app: "shop".into(),
            operation: scotty_core::apps::app_data::AppOperation {
                operation: "rebuild".into(),
                task_id: Some(task_id),
                started_at,
            },
        }
        .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"], true);
        let message = body["message"].as_str().unwrap();
        assert!(
            message.contains("shop")
                && message.contains("rebuild")
                && message.contains(&task_id.to_string()),
            "{message}"
        );
        assert_eq!(body["operation"]["operation"], "rebuild");
        assert_eq!(body["operation"]["task_id"], task_id.to_string());
        let started: jiff::Timestamp =
            serde_json::from_value(body["operation"]["started_at"].clone()).unwrap();
        assert_eq!(started, started_at);
    }

    #[test]
    fn client_errors_are_not_500() {
        let status = |e: AppError| e.get_error_msg().0;
        assert_eq!(
            status(AppError::CantDestroyUnmanagedApp("shop".into())),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status(AppError::OperationNotSupportedForLegacyApp("shop".into())),
            StatusCode::CONFLICT
        );
        for e in [
            AppError::InvalidInput,
            AppError::FileContentDecodingError,
            AppError::NoDockerComposeFile,
            AppError::InvalidDockerComposeFile,
            AppError::PublicServiceNotFound("web".into()),
            AppError::PublicPortsNotSupported("web".into()),
            AppError::EnvironmentVariablesNotSupported("FOO".into()),
            AppError::RegistryNotFound("r".into()),
            AppError::AppBlueprintNotFound("b".into()),
            AppError::AppBlueprintMismatch("b".into()),
            AppError::InvalidNotificationServiceIds("x".into()),
        ] {
            assert_eq!(status(e.clone()), StatusCode::BAD_REQUEST, "{e}");
        }
    }
}
