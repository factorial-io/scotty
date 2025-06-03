use axum::{response::IntoResponse, Json};
use scotty_core::{
    apps::{
        app_data::{AppData, AppSettings},
        shared_app_list::AppDataVec,
    },
    tasks::running_app_context::RunningAppContext,
    utils::sensitive_data::mask_sensitive_env_map,
};

/// A wrapper around Axum's Json response type that masks sensitive environment variables
/// in API responses containing AppData, AppDataVec, or AppSettings.
///
/// This ensures sensitive data is only masked when responding through the API,
/// and not in other parts of the application where the raw values are needed.
///
/// Note: SecureJson should only be used with AppData, AppDataVec, and AppSettings types.
/// For other types, use Json directly.
#[derive(Debug, Clone)]
pub struct SecureJson<T>(pub T);

/// Implementation for AppData
impl IntoResponse for SecureJson<AppData> {
    fn into_response(self) -> axum::response::Response {
        let mut app_data = self.0;

        // Mask sensitive environment variables if settings exist
        if let Some(settings) = app_data.settings.as_mut() {
            let masked_env = mask_sensitive_env_map(&settings.environment);
            settings.environment = masked_env;
        }

        Json(app_data).into_response()
    }
}

/// Implementation for AppDataVec (list of apps)
impl IntoResponse for SecureJson<AppDataVec> {
    fn into_response(self) -> axum::response::Response {
        let mut apps_vec = self.0;

        // Process each app in the vector
        for app in &mut apps_vec.apps {
            if let Some(settings) = app.settings.as_mut() {
                let masked_env = mask_sensitive_env_map(&settings.environment);
                settings.environment = masked_env;
            }
        }

        Json(apps_vec).into_response()
    }
}

/// Implementation for AppSettings directly
impl IntoResponse for SecureJson<AppSettings> {
    fn into_response(self) -> axum::response::Response {
        let mut settings = self.0;
        let masked_env = mask_sensitive_env_map(&settings.environment);
        settings.environment = masked_env;

        Json(settings).into_response()
    }
}

/// Implementation for RunningAppContext
impl IntoResponse for SecureJson<RunningAppContext> {
    fn into_response(self) -> axum::response::Response {
        let mut running_context = self.0;

        // Mask sensitive environment variables if settings exist
        if let Some(settings) = running_context.app_data.settings.as_mut() {
            let masked_env = mask_sensitive_env_map(&settings.environment);
            settings.environment = masked_env;
        }

        Json(running_context).into_response()
    }
}
