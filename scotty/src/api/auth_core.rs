use subtle::ConstantTimeEq;
use tracing::{debug, warn};

use crate::app_state::SharedAppState;
use scotty_core::settings::api_server::{AuthMode, DEFAULT_DEV_USER_EMAIL};

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub email: String,
    pub name: String,
    #[allow(dead_code)] // Future use for profile pictures
    pub picture: Option<String>,
    #[allow(dead_code)] // Used for OAuth token forwarding in future implementations
    pub access_token: Option<String>,
}

/// Authenticate a user from a token based on the configured auth mode
///
/// Accepts tokens with or without "Bearer " prefix (prefix is stripped if present).
///
/// Authentication flow by mode:
/// - Development: No token needed, returns dev user
/// - Bearer: Validates against configured bearer tokens (fast HashMap lookup)
/// - OAuth: Tries bearer tokens first (fast), falls back to OIDC validation (network call)
pub async fn authenticate_user_from_token(
    state: &SharedAppState,
    token: &str,
) -> anyhow::Result<CurrentUser> {
    // Strip "Bearer " prefix if present - accept both formats
    let raw_token = token.strip_prefix("Bearer ").unwrap_or(token);

    match state.settings.api.auth_mode {
        AuthMode::Development => Ok(authenticate_dev_user(state)),
        AuthMode::Bearer => {
            // Use warning-level logging for Bearer mode (failures are unexpected)
            match authorize_bearer_user(state.clone(), raw_token, true).await {
                Some(user) => Ok(user),
                None => Err(anyhow::anyhow!("Invalid bearer token")),
            }
        }
        AuthMode::OAuth => {
            // Try bearer token authentication first (fast HashMap lookup)
            // This avoids network latency for service accounts
            // Use debug-level logging since this is a fallback check before OAuth
            if let Some(user) = authorize_bearer_user(state.clone(), raw_token, false).await {
                return Ok(user);
            }

            // Fall back to OAuth validation (network call to OIDC provider)
            match authorize_oauth_user_native(state.clone(), raw_token).await {
                Some(user) => Ok(user),
                None => Err(anyhow::anyhow!("Invalid OAuth token")),
            }
        }
    }
}

/// Create a development mode user
pub fn authenticate_dev_user(state: &SharedAppState) -> CurrentUser {
    CurrentUser {
        email: state
            .settings
            .api
            .dev_user_email
            .clone()
            .unwrap_or_else(|| DEFAULT_DEV_USER_EMAIL.to_string()),
        name: state
            .settings
            .api
            .dev_user_name
            .clone()
            .unwrap_or_else(|| "Dev User".to_string()),
        picture: None,
        access_token: None,
    }
}

/// Authorize a bearer token user
///
/// Performs reverse lookup to find token identifier, then looks up user assignments
/// by identifier in the authorization service.
///
/// # Arguments
/// * `shared_app_state` - Application state with settings and auth service
/// * `token` - Raw token value (without "Bearer " prefix)
/// * `log_failures_as_warnings` - If true, log failures as warnings. If false, use debug level.
///   Set to true for Bearer auth mode, false for OAuth fallback mode.
pub async fn authorize_bearer_user(
    shared_app_state: SharedAppState,
    token: &str,
    log_failures_as_warnings: bool,
) -> Option<CurrentUser> {
    // Reverse lookup: find which identifier maps to this token
    let identifier = match find_token_identifier(&shared_app_state, token) {
        Some(id) => id,
        None => {
            if log_failures_as_warnings {
                warn!("Bearer token authentication failed - token not found in bearer_tokens configuration (token starts with: {}...)",
                      token.chars().take(8).collect::<String>());
            } else {
                debug!(
                    "Token not found in bearer_tokens configuration (token starts with: {}...)",
                    token.chars().take(8).collect::<String>()
                );
            }
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
    if log_failures_as_warnings {
        warn!(
            "Bearer token authentication failed - identifier '{}' not found in RBAC assignments",
            identifier
        );
    } else {
        debug!("Identifier '{}' not found in RBAC assignments", identifier);
    }
    None
}

/// Native OAuth token validation
///
/// # Arguments
/// * `shared_app_state` - Application state with OAuth client
/// * `token` - Raw token value (without "Bearer " prefix)
pub async fn authorize_oauth_user_native(
    shared_app_state: SharedAppState,
    token: &str,
) -> Option<CurrentUser> {
    debug!("Validating OAuth token");

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

/// Find the token identifier by reverse-looking up the actual token
///
/// Uses constant-time comparison to prevent timing attacks that could reveal
/// valid tokens through response time measurements.
fn find_token_identifier(shared_app_state: &SharedAppState, token: &str) -> Option<String> {
    // Search through configured bearer tokens to find matching identifier
    for (identifier, configured_token) in &shared_app_state.settings.api.bearer_tokens {
        // Use constant-time comparison to prevent timing attacks
        if token.as_bytes().ct_eq(configured_token.as_bytes()).into() {
            return Some(identifier.clone());
        }
    }

    None
}
