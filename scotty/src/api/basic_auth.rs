use axum::{
    extract::{Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{debug, warn};

use crate::app_state::SharedAppState;
use scotty_core::settings::api_server::AuthMode;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub email: String,
    pub name: String,
    #[allow(dead_code)] // Used for OAuth token forwarding in future implementations
    pub access_token: Option<String>,
}

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
            Some(CurrentUser {
                email: state
                    .settings
                    .api
                    .dev_user_email
                    .clone()
                    .unwrap_or_else(|| "dev@localhost".to_string()),
                name: state
                    .settings
                    .api
                    .dev_user_name
                    .clone()
                    .unwrap_or_else(|| "Dev User".to_string()),
                access_token: None,
            })
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
            debug!("Using bearer token auth mode");
            if state.settings.api.access_token.is_none() {
                debug!("No access token configured, allowing request");
                return Ok(next.run(req).await);
            }

            let auth_header = req
                .headers()
                .get(http::header::AUTHORIZATION)
                .and_then(|header| header.to_str().ok());

            let auth_header = if let Some(auth_header) = auth_header {
                auth_header
            } else {
                warn!("Missing Authorization header in bearer mode");
                return Err(StatusCode::UNAUTHORIZED);
            };

            authorize_bearer_user(state, auth_header).await
        }
    };

    if let Some(user) = current_user {
        debug!("User authenticated: {} <{}>", user.name, user.email);
        req.extensions_mut().insert(user);
        Ok(next.run(req).await)
    } else {
        warn!("Authentication failed");
        Err(StatusCode::UNAUTHORIZED)
    }
}

// Legacy function for oauth2-proxy compatibility (kept for backward compatibility)
#[allow(dead_code)]
fn authorize_oauth_user(req: &Request) -> Option<CurrentUser> {
    let headers = req.headers();

    let email = headers
        .get("X-Auth-Request-Email")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let user = headers
        .get("X-Auth-Request-User")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let access_token = headers
        .get("X-Auth-Request-Access-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    match (email, user) {
        (Some(email), Some(name)) => {
            debug!("OAuth user found: {} <{}>", name, email);
            Some(CurrentUser {
                email,
                name,
                access_token,
            })
        }
        _ => {
            warn!("Missing OAuth headers from proxy");
            None
        }
    }
}

// Native OAuth token validation
async fn authorize_oauth_user_native(
    shared_app_state: SharedAppState,
    auth_header: &str,
) -> Option<CurrentUser> {
    // Extract Bearer token
    let token = auth_header.strip_prefix("Bearer ")?;

    debug!("Validating OAuth Bearer token");

    // Get OAuth client for token validation
    let oauth_state = shared_app_state.oauth_state.as_ref()?;

    match oauth_state.client.validate_oidc_token(token).await {
        Ok(oidc_user) => {
            debug!(
                "OAuth token validated for user: {} <{}>",
                oidc_user.name.as_deref().unwrap_or("Unknown"),
                oidc_user.email.as_deref().unwrap_or("unknown@example.com")
            );
            Some(CurrentUser {
                email: oidc_user.email.unwrap_or("unknown@example.com".to_string()),
                name: oidc_user.name.unwrap_or("Unknown".to_string()),
                access_token: Some(token.to_string()),
            })
        }
        Err(e) => {
            warn!("OAuth token validation failed: {}", e);
            None
        }
    }
}

pub async fn authorize_bearer_user(
    shared_app_state: SharedAppState,
    auth_token: &str,
) -> Option<CurrentUser> {
    let required_token = shared_app_state.settings.api.access_token.as_ref().unwrap();
    auth_token
        .strip_prefix("Bearer ")
        .filter(|token| token == required_token)
        .map(|token| CurrentUser {
            email: "api-user@localhost".to_string(),
            name: "API User".to_string(),
            access_token: Some(token.to_string()),
        })
}
