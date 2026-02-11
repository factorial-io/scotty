#![allow(dead_code)]

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

    #[error("App not found: {0}")]
    AppNotFound(String),

    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("File content could not be decoded!")]
    FileContentDecodingError,

    #[error("File compression corrupted for {0}: {1}")]
    FileCompressionCorrupted(String, String),

    #[error("File {0} exceeds maximum decompressed size of {1} bytes")]
    FileDecompressedSizeExceeded(String, usize),

    #[error("Cant destroy an unmanaged app: {0}")]
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
            AppError::FileCompressionCorrupted(_, _) => StatusCode::BAD_REQUEST,
            AppError::FileDecompressedSizeExceeded(_, _) => StatusCode::BAD_REQUEST,
            AppError::CantAdoptAppWithExistingSettings(_) => StatusCode::BAD_REQUEST,
            AppError::MiddlewareNotAllowed(_) => StatusCode::BAD_REQUEST,
            AppError::AppNotRunning(_) => StatusCode::CONFLICT,
            AppError::ActionNotFound(_) => StatusCode::NOT_FOUND,
            AppError::ActionNotExecutable(_) => StatusCode::FORBIDDEN,
            AppError::ActionAlreadyExists(_) => StatusCode::CONFLICT,
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
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        if let Some(app_error) = e.downcast_ref::<AppError>() {
            return app_error.clone();
        }
        AppError::InternalServerError(e.to_string())
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
                let body = serde_json::json!({ "error": true, "message": body });
                (status, Json(body)).into_response()
            }
        }
    }
}
