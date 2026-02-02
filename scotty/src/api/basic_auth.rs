use axum::{
    extract::{Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

use crate::api::auth_core::authenticate_user_from_token;
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

    // Extract token based on auth mode
    // - Development: no token needed (empty string)
    // - Bearer/OAuth: extract from Authorization header
    let token: &str = match state.settings.api.auth_mode {
        AuthMode::Development => "",
        AuthMode::Bearer | AuthMode::OAuth => {
            match req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok())
            {
                Some(header) => header,
                None => {
                    warn!(
                        "Missing Authorization header | {} {} | auth_mode: {:?}",
                        req.method(),
                        req.uri(),
                        state.settings.api.auth_mode
                    );
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    };

    // Use centralized authentication for all modes
    match authenticate_user_from_token(&state, token).await {
        Ok(user) => {
            debug!("User authenticated: {} <{}>", user.name, user.email);
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        Err(e) => {
            warn!(
                "Authentication failed for {} {} | auth_mode: {:?} | error: {}",
                req.method(),
                req.uri(),
                state.settings.api.auth_mode,
                e
            );
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
