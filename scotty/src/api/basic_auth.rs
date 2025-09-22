use axum::{
    extract::{Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

use crate::api::auth_core::{
    authenticate_dev_user, authorize_bearer_user, authorize_oauth_user_native,
};
use crate::app_state::SharedAppState;

// Re-export CurrentUser for backward compatibility
pub use crate::api::auth_core::CurrentUser;
use scotty_core::settings::api_server::AuthMode;

pub async fn auth(
    State(state): State<SharedAppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    debug!(
        "Auth middleware triggered with mode: {:?}",
        state.settings.api.auth_mode
    );

    let current_user = match state.settings.api.auth_mode {
        AuthMode::Development => {
            debug!("Using development auth mode");
            Some(authenticate_dev_user(&state))
        }
        AuthMode::OAuth => {
            debug!("Using OAuth auth mode with native tokens");
            let auth_header = req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok());

            let auth_header = if let Some(auth_header) = auth_header {
                auth_header
            } else {
                warn!("Missing Authorization header in OAuth mode");
                return Err(StatusCode::UNAUTHORIZED);
            };

            authorize_oauth_user_native(state.clone(), auth_header).await
        }
        AuthMode::Bearer => {
            debug!("Using bearer token auth mode with RBAC");

            let auth_header = req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok());

            let auth_header = if let Some(auth_header) = auth_header {
                auth_header
            } else {
                warn!(
                    "Missing Authorization header in bearer mode | {} {} | user_agent: {:?}",
                    req.method(),
                    req.uri(),
                    req.headers()
                        .get("user-agent")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("unknown")
                );
                return Err(StatusCode::UNAUTHORIZED);
            };

            authorize_bearer_user(state.clone(), auth_header).await
        }
    };

    if let Some(user) = current_user {
        debug!("User authenticated: {} <{}>", user.name, user.email);
        req.extensions_mut().insert(user);
        Ok(next.run(req).await)
    } else {
        let method = req.method();
        let uri = req.uri();
        let auth_mode = &state.settings.api.auth_mode;
        let has_auth_header = req.headers().contains_key(http::header::AUTHORIZATION);

        warn!(
            "Authentication failed for {} {} | auth_mode: {:?} | has_auth_header: {} | user_agent: {:?}", 
            method,
            uri,
            auth_mode,
            has_auth_header,
            req.headers().get("user-agent").and_then(|h| h.to_str().ok()).unwrap_or("unknown")
        );
        Err(StatusCode::UNAUTHORIZED)
    }
}
