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
    #[allow(dead_code)] // Future use for profile pictures
    pub picture: Option<String>,
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
                picture: None,
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
                    req.headers().get("user-agent").and_then(|h| h.to_str().ok()).unwrap_or("unknown")
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
                picture: None,
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
    let token = match auth_header.strip_prefix("Bearer ") {
        Some(token) => token,
        None => {
            warn!("OAuth authentication failed - invalid Authorization header format (expected 'Bearer <token>', got: {}...)", 
                  auth_header.chars().take(20).collect::<String>());
            return None;
        }
    };

    debug!("Validating OAuth Bearer token");

    // Get OAuth client for token validation
    let oauth_state = match shared_app_state.oauth_state.as_ref() {
        Some(state) => state,
        None => {
            warn!("OAuth authentication failed - OAuth state not initialized (server may not be configured for OAuth)");
            return None;
        }
    };

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
                picture: oidc_user.picture,
                access_token: Some(token.to_string()),
            })
        }
        Err(e) => {
            warn!("OAuth token validation failed: {}", e);
            None
        }
    }
}

/// Authorize a bearer token user
///
/// Performs reverse lookup to find token identifier, then looks up user assignments
/// by identifier in the authorization service.
pub async fn authorize_bearer_user(
    shared_app_state: SharedAppState,
    auth_token: &str,
) -> Option<CurrentUser> {
    // Extract Bearer token
    let token = match auth_token.strip_prefix("Bearer ") {
        Some(token) => token,
        None => {
            warn!("Bearer token authentication failed - invalid Authorization header format (expected 'Bearer <token>', got: {}...)", 
                  auth_token.chars().take(20).collect::<String>());
            return None;
        }
    };

    // Reverse lookup: find which identifier maps to this token
    let identifier = match find_token_identifier(&shared_app_state, token) {
        Some(id) => id,
        None => {
            warn!("Bearer token authentication failed - token not found in bearer_tokens configuration (token starts with: {}...)", 
                  token.chars().take(8).collect::<String>());
            return None;
        }
    };
    debug!("Found identifier '{}' for bearer token", identifier);

    // Look up the user by identifier in authorization service
    let auth_service = &shared_app_state.auth_service;
    let user_id = format!("identifier:{}", identifier);

    if let Some(_user_info) = auth_service.get_user_by_identifier(&user_id).await {
        debug!("Found user assignments for identifier: {}", identifier);
        return Some(CurrentUser {
            email: format!("identifier:{}", identifier), // Use identifier format for user.email
            name: format!("Token User ({})", identifier),
            picture: None,
            access_token: Some(token.to_string()),
        });
    }

    // Identifier not found in RBAC assignments
    warn!(
        "Bearer token authentication failed - identifier '{}' not found in RBAC assignments",
        identifier
    );
    None
}

/// Find the token identifier by reverse-looking up the actual token
fn find_token_identifier(shared_app_state: &SharedAppState, token: &str) -> Option<String> {
    // Search through configured bearer tokens to find matching identifier
    for (identifier, configured_token) in &shared_app_state.settings.api.bearer_tokens {
        if configured_token == token {
            return Some(identifier.clone());
        }
    }

    None
}
