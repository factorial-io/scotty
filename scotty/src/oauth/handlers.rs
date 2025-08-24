use super::{
    DeviceFlowStore, OAuthClient, OAuthSession, OAuthSessionStore, WebFlowSession, WebFlowStore,
};
use crate::api::error::AppError;
use crate::app_state::SharedAppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Redirect},
};
use base64::{engine::general_purpose, Engine as _};
use oauth2::{AuthorizationCode, CsrfToken, PkceCodeVerifier};
use serde::Deserialize;
use std::time::{Duration, SystemTime};
use tracing::{debug, error};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OAuthState {
    pub client: OAuthClient,
    pub device_flow_store: DeviceFlowStore,
    pub web_flow_store: WebFlowStore,
    pub session_store: OAuthSessionStore,
}

// Re-export the shared OAuth types
pub use scotty_core::auth::{
    DeviceFlowResponse, DeviceTokenQuery, ErrorResponse, OAuthError, TokenResponse,
};

/// Start OAuth device flow
#[utoipa::path(
    post,
    path = "/oauth/device",
    responses(
        (status = 200, description = "Device flow started", body = DeviceFlowResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn start_device_flow(
    State(app_state): State<SharedAppState>,
) -> Result<Json<DeviceFlowResponse>, AppError> {
    debug!("Starting device flow");

    let oauth_state = match &app_state.oauth_state {
        Some(state) => state,
        None => return Err(OAuthError::OauthNotConfigured.into()),
    };

    match oauth_state
        .client
        .start_device_flow(oauth_state.device_flow_store.clone())
        .await
    {
        Ok(session) => {
            let expires_in = session
                .expires_at
                .duration_since(std::time::SystemTime::now())
                .unwrap_or_default()
                .as_secs();

            Ok(Json(DeviceFlowResponse {
                device_code: session.device_code,
                user_code: session.user_code,
                verification_uri: session.verification_uri,
                verification_uri_complete: None,
                expires_in,
                interval: Some(5), // Poll every 5 seconds
            }))
        }
        Err(e) => {
            error!("Failed to start device flow: {}", e);
            Err(AppError::InternalServerError(format!(
                "Failed to start device flow: {}",
                e
            )))
        }
    }
}

/// Poll for device flow token
#[utoipa::path(
    post,
    path = "/oauth/device/token",
    params(DeviceTokenQuery),
    responses(
        (status = 200, description = "Token obtained", body = TokenResponse),
        (status = 400, description = "Authorization pending or denied", body = AppError),
        (status = 404, description = "Session not found", body = AppError),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn poll_device_token(
    State(app_state): State<SharedAppState>,
    Query(params): Query<DeviceTokenQuery>,
) -> Result<Json<TokenResponse>, AppError> {
    debug!("Polling device token for: {}", params.device_code);

    let oauth_state = match &app_state.oauth_state {
        Some(state) => state,
        None => return Err(OAuthError::OauthNotConfigured.into()),
    };

    match oauth_state
        .client
        .poll_device_token(&params.device_code, oauth_state.device_flow_store.clone())
        .await
    {
        Ok(oidc_token) => {
            // Validate the OIDC token and get user info
            match oauth_state.client.validate_oidc_token(&oidc_token).await {
                Ok(user) => {
                    // For now, we'll return the OIDC token as the access token
                    // In a full implementation, you might want to create a Scotty session token
                    Ok(Json(TokenResponse {
                        access_token: oidc_token,
                        token_type: "Bearer".to_string(),
                        user_id: user.username.clone().unwrap_or(user.id.clone()),
                        user_name: user.name.unwrap_or("Unknown".to_string()),
                        user_email: user.email.unwrap_or("unknown@example.com".to_string()),
                        refresh_token: None,
                        expires_in: None,
                    }))
                }
                Err(e) => {
                    error!("Failed to validate OIDC token: {}", e);
                    Err(OAuthError::ServerError("Failed to validate token".to_string()).into())
                }
            }
        }
        Err(OAuthError::AuthorizationPending) => Err(OAuthError::AuthorizationPending.into()),
        Err(OAuthError::SlowDown) => Err(OAuthError::SlowDown.into()),
        Err(OAuthError::AccessDenied) => Err(OAuthError::AccessDenied.into()),
        Err(OAuthError::SessionNotFound) => Err(OAuthError::SessionNotFound.into()),
        Err(OAuthError::ExpiredSession) => Err(OAuthError::ExpiredToken.into()),
        Err(e) => {
            error!("Device flow error: {}", e);
            Err(OAuthError::ServerError(format!("OAuth error: {}", e)).into())
        }
    }
}

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
    {
        Ok((auth_url, pkce_verifier)) => {
            // Store session for later verification
            let session = WebFlowSession {
                csrf_token: csrf_token_raw.secret().clone(), // Store only the raw CSRF token part
                pkce_verifier: general_purpose::STANDARD.encode(pkce_verifier.secret()), // Store PKCE verifier
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

        if csrf_part != session.csrf_token {
            error!("CSRF token mismatch");
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::from(OAuthError::InvalidState)),
            )
                .into_response();
        }
    }

    // Exchange code for token
    let pkce_verifier = match general_purpose::STANDARD.decode(&session.pkce_verifier) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(verifier_str) => PkceCodeVerifier::new(verifier_str),
            Err(e) => {
                error!("Failed to decode PKCE verifier: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::from(OAuthError::ServerError(
                        "Invalid PKCE verifier".to_string(),
                    ))),
                )
                    .into_response();
            }
        },
        Err(e) => {
            error!("Failed to decode PKCE verifier: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::from(OAuthError::ServerError(
                    "Invalid PKCE verifier".to_string(),
                ))),
            )
                .into_response();
        }
    };

    debug!(
        "Using redirect URL for token exchange: {}",
        session.redirect_url
    );
    match oauth_state
        .client
        .exchange_code_for_token(code, session.redirect_url.clone(), pkce_verifier)
        .await
    {
        Ok(access_token) => {
            // Validate token and get user info
            match oauth_state.client.validate_oidc_token(&access_token).await {
                Ok(user) => {
                    // Clean up session
                    {
                        let mut sessions = oauth_state.web_flow_store.lock().unwrap();
                        sessions.remove(&session_id);
                    }

                    debug!(
                        "OAuth web flow completed successfully for user: {}",
                        user.username.as_deref().unwrap_or(&user.id)
                    );

                    // Create OAuth session for token exchange
                    let oauth_session_id = Uuid::new_v4().to_string();
                    let oauth_session = OAuthSession {
                        oidc_token: access_token,
                        user: user.clone(),
                        expires_at: SystemTime::now() + Duration::from_secs(300), // 5 minutes
                    };

                    // Store the session
                    {
                        let mut sessions = oauth_state.session_store.lock().unwrap();
                        sessions.insert(oauth_session_id.clone(), oauth_session);
                    }

                    // Redirect to frontend with session ID
                    let frontend_url =
                        if let Some(frontend_callback) = &session.frontend_callback_url {
                            format!("{}?session_id={}", frontend_callback, oauth_session_id)
                        } else {
                            // Fallback to frontend OAuth callback page if no frontend callback specified
                            format!(
                                "http://localhost:21342/oauth/callback?session_id={}",
                                oauth_session_id
                            )
                        };

                    debug!("Redirecting to frontend: {}", frontend_url);
                    Redirect::temporary(&frontend_url).into_response()
                }
                Err(e) => {
                    error!("Failed to validate OIDC token: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse::from(OAuthError::ServerError(
                            "Failed to validate token".to_string(),
                        ))),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to exchange code for token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::from(OAuthError::ServerError(format!(
                    "Token exchange failed: {}",
                    e
                )))),
            )
                .into_response()
        }
    }
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

    // Create meaningful fallback values based on available OIDC data
    let display_name = session
        .user
        .name
        .clone()
        .or_else(|| session.user.username.clone())
        .unwrap_or_else(|| format!("User {}", &session.user.id[..8.min(session.user.id.len())]));

    let display_email = session.user.email.clone().unwrap_or_else(|| {
        format!(
            "{}@oidc-provider.local",
            session.user.username.as_deref().unwrap_or("user")
        )
    });

    debug!(
        "Session exchange successful for user: {} <{}>",
        display_name, display_email
    );

    // For now, return the OIDC token directly
    // TODO: Generate a Scotty JWT token instead
    Ok(axum::response::Json(TokenResponse {
        access_token: session.oidc_token,
        token_type: "Bearer".to_string(),
        user_id: session
            .user
            .username
            .clone()
            .unwrap_or(session.user.id.clone()),
        user_name: display_name,
        user_email: display_email,
        refresh_token: None,
        expires_in: None,
    }))
}
