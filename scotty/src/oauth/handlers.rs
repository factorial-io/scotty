use super::server::WEB_CLIENT_ID;
use super::{OAuthClient, OAuthSession, OAuthSessionStore, WebFlowSession, WebFlowStore};
use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect},
};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeVerifier};
use openidconnect::Nonce;
use scotty_core::utils::secret::MaskedSecret;
use serde::Deserialize;
use std::time::{Duration, SystemTime};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OAuthState {
    pub client: OAuthClient,
    pub web_flow_store: WebFlowStore,
    pub session_store: OAuthSessionStore,
}

// Re-export the shared OAuth types
pub use scotty_core::auth::{DeviceFlowResponse, ErrorResponse, OAuthError, TokenResponse};

#[derive(Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct AuthorizeQuery {
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SessionExchangeRequest {
    pub session_id: String,
}

/// Start OAuth web authorization flow
#[utoipa::path(
    get,
    path = "/oauth/authorize",
    params(AuthorizeQuery),
    responses(
        (status = 302, description = "Redirect to GitLab OAuth"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn start_authorization_flow(
    State(app_state): State<SharedAppState>,
    Query(params): Query<AuthorizeQuery>,
) -> impl IntoResponse {
    super::metrics::record_web_flow_start();
    debug!("Starting OAuth authorization flow");

    let oauth_state = match &app_state.oauth_state {
        Some(state) => state,
        None => return AppError::OAuthError(OAuthError::OauthNotConfigured).into_response(),
    };

    // Generate session ID and CSRF token separately
    let session_id = Uuid::new_v4().to_string();
    let csrf_token_raw = CsrfToken::new_random();
    let csrf_token = CsrfToken::new(format!("{}:{}", session_id, csrf_token_raw.secret()));

    // Store frontend callback URL separately before consuming params.redirect_uri
    let frontend_callback_url = params.redirect_uri.clone();

    // Determine redirect URL - use configured URL from settings
    let redirect_url = params
        .redirect_uri
        .unwrap_or_else(|| app_state.settings.api.oauth.redirect_url.clone());

    debug!("Using redirect URL for authorization: {}", redirect_url);

    // Generate authorization URL with PKCE
    match oauth_state
        .client
        .get_authorization_url(redirect_url.clone(), csrf_token.clone())
        .await
    {
        Ok((auth_url, pkce_verifier, nonce)) => {
            // Store session for later verification
            let session = WebFlowSession {
                csrf_token: MaskedSecret::new(csrf_token_raw.secret().clone()), // Store only the raw CSRF token part (protected)
                pkce_verifier: MaskedSecret::new(pkce_verifier.secret().to_string()), // Store PKCE verifier (protected)
                nonce: MaskedSecret::new(nonce.secret().to_string()),
                redirect_url: redirect_url.clone(), // OAuth redirect URL for token exchange
                frontend_callback_url,              // Frontend callback URL
                expires_at: SystemTime::now() + Duration::from_secs(600), // 10 minutes
            };

            {
                let mut sessions = oauth_state.web_flow_store.lock().unwrap();
                sessions.insert(session_id.clone(), session);
            }

            debug!("Redirecting to GitLab OAuth: {}", auth_url);
            Redirect::temporary(auth_url.as_str()).into_response()
        }
        Err(e) => {
            error!("Failed to generate authorization URL: {}", e);
            AppError::InternalServerError(format!("Failed to start authorization: {}", e))
                .into_response()
        }
    }
}

#[derive(Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    #[allow(dead_code)]
    pub session_id: Option<String>,
}

/// Handle OAuth callback from GitLab
#[utoipa::path(
    get,
    path = "/api/oauth/callback",
    params(CallbackQuery),
    responses(
        (status = 200, description = "OAuth callback handled", body = TokenResponse),
        (status = 400, description = "OAuth error", body = AppError),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn handle_oauth_callback(
    State(app_state): State<SharedAppState>,
    Query(params): Query<CallbackQuery>,
) -> impl IntoResponse {
    debug!(
        "Handling OAuth callback with params: code={:?}, state={:?}, error={:?}",
        params.code.as_ref().map(|_| "[REDACTED]"),
        params
            .state
            .as_ref()
            .map(|s| &s[..std::cmp::min(10, s.len())]),
        params.error
    );

    let oauth_state = match &app_state.oauth_state {
        Some(state) => state,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::from(OAuthError::OauthNotConfigured)),
            )
                .into_response();
        }
    };

    // Check for OAuth errors
    if let Some(error) = params.error {
        error!("OAuth authorization failed: {}", error);
        let description = params
            .error_description
            .unwrap_or_else(|| "Unknown error".to_string());
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error,
                error_description: Some(description),
            }),
        )
            .into_response();
    }

    // Extract session ID from state parameter and authorization code
    let state = params.state.as_ref().ok_or_else(|| {
        error!("Missing state parameter in callback");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::from(OAuthError::InvalidRequest(
                "Missing state parameter".to_string(),
            ))),
        )
    });

    let state = match state {
        Ok(s) => s,
        Err(response) => return response.into_response(),
    };

    // Extract session ID from state (format: "session_id:csrf_token")
    let session_id = if let Some((session_id, _)) = state.split_once(':') {
        session_id.to_string()
    } else {
        error!("Invalid state format in callback");
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::from(OAuthError::InvalidRequest(
                "Invalid state format".to_string(),
            ))),
        )
            .into_response();
    };

    let code = params.code.ok_or_else(|| {
        error!("Missing authorization code in callback");
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::from(OAuthError::InvalidRequest(
                "Missing authorization code".to_string(),
            ))),
        )
    });

    let code = match code {
        Ok(c) => AuthorizationCode::new(c),
        Err(response) => return response.into_response(),
    };

    // Retrieve and validate session
    let session = {
        let sessions = oauth_state.web_flow_store.lock().unwrap();
        sessions.get(&session_id).cloned()
    };

    let session = match session {
        Some(session) => {
            if SystemTime::now() > session.expires_at {
                error!("OAuth session expired");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from(OAuthError::ExpiredSession)),
                )
                    .into_response();
            }
            session
        }
        None => {
            error!("OAuth session not found");
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::from(OAuthError::SessionNotFound)),
            )
                .into_response();
        }
    };

    // Validate CSRF state - extract just the CSRF part from the state parameter
    if let Some(state) = &params.state {
        // State format is "session_id:csrf_token", extract the CSRF part
        let csrf_part = if let Some((_, csrf_token)) = state.split_once(':') {
            csrf_token
        } else {
            error!("Invalid state format for CSRF validation");
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::from(OAuthError::InvalidState)),
            )
                .into_response();
        };

        if csrf_part != session.csrf_token.expose_secret() {
            error!("CSRF token mismatch");
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::from(OAuthError::InvalidState)),
            )
                .into_response();
        }
    }

    // Session is single-use from here on.
    {
        let mut sessions = oauth_state.web_flow_store.lock().unwrap();
        sessions.remove(&session_id);
    }

    let session_server = match &app_state.session_server {
        Some(server) => server,
        None => {
            error!("OAuth login without a session server (no oauth database or jwt_secret)");
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::from(OAuthError::OauthNotConfigured)),
            )
                .into_response();
        }
    };

    // Exchange the code and read the identity from the verified ID token.
    let pkce_verifier = PkceCodeVerifier::new(session.pkce_verifier.expose_secret().to_string());
    let nonce = Nonce::new(session.nonce.expose_secret().to_string());
    debug!(
        "Using redirect URL for token exchange: {}",
        session.redirect_url
    );
    let identity = match oauth_state
        .client
        .login_identity(code, session.redirect_url.clone(), pkce_verifier, nonce)
        .await
    {
        Ok(identity) => identity,
        Err(e) => {
            error!("OAuth login failed: {}", e);
            return (StatusCode::from(e.clone()), Json(ErrorResponse::from(e))).into_response();
        }
    };
    // login_identity guarantees an email.
    let email = identity.email.clone().unwrap_or_default();

    // Issue the Scotty session: a grant for the email, no refresh token.
    let tokens = match session_server
        .issue_grant(&email, WEB_CLIENT_ID, "Scotty web", "api")
        .await
    {
        Ok((_grant, tokens)) => tokens,
        Err(grantry::server::TokenError::InvalidGrant) => {
            error!("OAuth login refused for {}: no role", email);
            let e = OAuthError::AccessDenied;
            return (StatusCode::from(e.clone()), Json(ErrorResponse::from(e))).into_response();
        }
        Err(e) => {
            error!("Failed to issue session for {}: {}", email, e);
            let e = OAuthError::ServerError("Failed to issue session".to_string());
            return (StatusCode::from(e.clone()), Json(ErrorResponse::from(e))).into_response();
        }
    };

    debug!("OAuth web flow completed for {}", email);

    let oauth_session_id = Uuid::new_v4().to_string();
    let oauth_session = OAuthSession {
        token: TokenResponse {
            access_token: tokens.access_token,
            token_type: tokens.token_type.to_string(),
            user_id: identity.user_id,
            user_name: identity.name.unwrap_or_else(|| email.clone()),
            user_email: email,
            user_picture: identity.picture,
            refresh_token: None,
            expires_in: Some(tokens.expires_in),
        },
        expires_at: SystemTime::now() + Duration::from_secs(300), // 5 minutes
    };
    {
        let mut sessions = oauth_state.session_store.lock().unwrap();
        sessions.insert(oauth_session_id.clone(), oauth_session);
    }

    // Redirect to frontend with session ID
    let frontend_url = if let Some(frontend_callback) = &session.frontend_callback_url {
        format!("{}?session_id={}", frontend_callback, oauth_session_id)
    } else {
        // Fallback to the public base URL (api.base_url)
        format!(
            "{}/oauth/callback?session_id={}",
            app_state.settings.api.public_base_url(),
            oauth_session_id
        )
    };

    debug!("Redirecting to frontend: {}", frontend_url);
    Redirect::temporary(&frontend_url).into_response()
}

/// Exchange OAuth session for bearer token
#[utoipa::path(
    post,
    path = "/oauth/exchange",
    request_body = SessionExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = TokenResponse),
        (status = 404, description = "Session not found", body = AppError),
        (status = 410, description = "Session expired", body = AppError),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn exchange_session_for_token(
    State(app_state): State<SharedAppState>,
    axum::extract::Json(request): axum::extract::Json<SessionExchangeRequest>,
) -> Result<axum::response::Json<TokenResponse>, AppError> {
    debug!("Exchanging session for token: {}", request.session_id);

    let oauth_state = match &app_state.oauth_state {
        Some(state) => state,
        None => return Err(OAuthError::OauthNotConfigured.into()),
    };

    // Retrieve and remove session (one-time use)
    let session = {
        let mut sessions = oauth_state.session_store.lock().unwrap();
        sessions.remove(&request.session_id)
    };

    let session = match session {
        Some(session) => {
            if SystemTime::now() > session.expires_at {
                error!("OAuth session expired: {}", request.session_id);
                return Err(OAuthError::ExpiredSession.into());
            }
            session
        }
        None => {
            error!("OAuth session not found: {}", request.session_id);
            return Err(OAuthError::SessionNotFound.into());
        }
    };

    debug!(
        "Session exchange successful for user: {} <{}>",
        session.token.user_name, session.token.user_email
    );

    Ok(axum::response::Json(session.token))
}
