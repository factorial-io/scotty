//! The CLI device login (design D3), brokered through the grantry session
//! server: `scottyctl` starts and polls at `/oauth/device(/token)`, a
//! logged-in user approves or denies in the frontend `/device` page.

use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use grantry::server::PollError;
use serde::{Deserialize, Serialize};
use tracing::{debug, error};

use super::server::{SessionServer, CLI_CLIENT_ID};
use crate::api::basic_auth::CurrentUser;
use crate::app_state::SharedAppState;
use scotty_core::auth::{DeviceFlowResponse, ErrorResponse, OAuthError, TokenResponse};

const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const CLI_CLIENT_NAME: &str = "scottyctl";
const CLI_SCOPE: &str = "api offline_access";

/// The session server, if the device login is on.
fn device_server(state: &SharedAppState) -> Option<&SessionServer> {
    state
        .session_server
        .as_ref()
        .filter(|_| state.settings.api.oauth.device_flow_enabled)
}

fn not_enabled() -> Response {
    oauth_error(
        StatusCode::NOT_FOUND,
        OAuthError::OauthNotConfigured.code(),
        "The device login is not enabled.",
    )
}

/// An RFC 6749 §5.2 error body.
fn oauth_error(status: StatusCode, code: &str, description: &str) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(ErrorResponse::from_string_with_description(
            code,
            description,
        )),
    )
        .into_response()
}

/// The caller's address as the proxy in front reports it; audit only.
fn client_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
}

/// Where users approve device logins.
fn verification_uri(state: &SharedAppState) -> String {
    let base = state
        .settings
        .api
        .oauth
        .frontend_base_url
        .as_deref()
        .map(|u| u.trim().trim_end_matches('/'))
        .filter(|u| !u.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| state.settings.api.public_base_url());
    format!("{base}/device")
}

/// Start a device login (RFC 8628 §3.1)
#[utoipa::path(
    post,
    path = "/oauth/device",
    responses(
        (status = 200, description = "Device login started", body = DeviceFlowResponse),
        (status = 404, description = "Device login not enabled", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn start_device_flow(State(state): State<SharedAppState>) -> Response {
    let Some(server) = device_server(&state) else {
        return not_enabled();
    };
    super::metrics::record_device_flow_start();
    let started = match server
        .start_device(CLI_CLIENT_ID, CLI_CLIENT_NAME, CLI_SCOPE)
        .await
    {
        Ok(started) => started,
        Err(e) => {
            error!("Failed to start device login: {}", e);
            return oauth_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                "Failed to start the device login.",
            );
        }
    };
    let verification_uri = verification_uri(&state);
    let user_code = started.device.user_code.clone();
    Json(DeviceFlowResponse {
        verification_uri_complete: Some(format!("{verification_uri}?code={user_code}")),
        verification_uri,
        expires_in: started.expires_in().max(0) as u64,
        interval: Some(started.device.interval_seconds.into()),
        device_code: started.device_code,
        user_code,
    })
    .into_response()
}

/// The request parameters of the token endpoint: the query string, overlaid
/// with a form or JSON body. scottyctl up to 0.x sends `device_code` in the
/// query and an empty JSON body.
fn token_params(
    query: HashMap<String, String>,
    headers: &HeaderMap,
    body: &[u8],
) -> HashMap<String, String> {
    let mut params = query;
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if content_type.starts_with("application/x-www-form-urlencoded") {
        params.extend(url::form_urlencoded::parse(body).into_owned());
    } else if content_type.starts_with("application/json") {
        if let Ok(json) = serde_json::from_slice::<HashMap<String, serde_json::Value>>(body) {
            params.extend(
                json.into_iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))),
            );
        }
    }
    params
}

/// Poll a device login or refresh a CLI session
///
/// One token endpoint with two grant types (RFC 6749 §4.5):
/// `urn:ietf:params:oauth:grant-type:device_code` with `device_code`
/// (RFC 8628 §3.4), and `refresh_token` with `refresh_token` (rotating).
/// A request with a `device_code` and no `grant_type` is a device poll.
#[utoipa::path(
    post,
    path = "/oauth/device/token",
    request_body(content = String, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Tokens issued", body = TokenResponse),
        (status = 400, description = "authorization_pending, slow_down, access_denied, expired_token, invalid_grant, invalid_request or unsupported_grant_type", body = ErrorResponse),
        (status = 404, description = "Device login not enabled", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "OAuth"
)]
pub async fn device_token(
    State(state): State<SharedAppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(server) = device_server(&state) else {
        return not_enabled();
    };
    let params = token_params(query, &headers, &body);
    let ip = client_ip(&headers);
    let grant_type = params.get("grant_type").map(String::as_str).or_else(|| {
        params
            .contains_key("device_code")
            .then_some(DEVICE_CODE_GRANT)
    });

    let result = match grant_type {
        Some(DEVICE_CODE_GRANT) => {
            let Some(device_code) = params.get("device_code") else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "device_code is required.",
                );
            };
            server
                .poll_device(device_code, CLI_CLIENT_ID, ip.as_deref())
                .await
        }
        Some("refresh_token") => {
            let Some(refresh_token) = params.get("refresh_token") else {
                return oauth_error(
                    StatusCode::BAD_REQUEST,
                    "invalid_request",
                    "refresh_token is required.",
                );
            };
            server
                .refresh(refresh_token, CLI_CLIENT_ID, ip.as_deref())
                .await
                .map_err(PollError::Token)
        }
        _ => {
            return oauth_error(
                StatusCode::BAD_REQUEST,
                "unsupported_grant_type",
                "grant_type must be the device code or refresh_token grant.",
            )
        }
    };

    match result {
        Ok((grant, tokens)) => {
            let email = grant.subject;
            (
                [(header::CACHE_CONTROL, "no-store")],
                Json(TokenResponse {
                    access_token: tokens.access_token,
                    token_type: tokens.token_type.to_string(),
                    user_id: email.clone(),
                    user_name: email.clone(),
                    user_email: email,
                    user_picture: None,
                    refresh_token: tokens.refresh_token,
                    expires_in: Some(tokens.expires_in),
                }),
            )
                .into_response()
        }
        Err(e) => match e.code() {
            Some(code) => {
                debug!("Device token request refused: {}", code);
                oauth_error(StatusCode::BAD_REQUEST, code, e.description())
            }
            None => {
                error!("Device token request failed: {}", e);
                oauth_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server_error",
                    "Internal error.",
                )
            }
        },
    }
}

/// A device login waiting for a decision, as the approval page shows it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeviceInfo {
    pub user_code: String,
    pub client_name: String,
    pub scope: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct DeviceDecision {
    pub user_code: String,
}

/// The device login server and the deciding user. Service accounts (bearer
/// tokens) decide nothing.
fn decider<'a>(
    state: &'a SharedAppState,
    user: &CurrentUser,
) -> Result<&'a SessionServer, StatusCode> {
    if user.email.starts_with("identifier:") {
        return Err(StatusCode::FORBIDDEN);
    }
    device_server(state).ok_or(StatusCode::NOT_FOUND)
}

fn backend_error(e: impl std::fmt::Display) -> StatusCode {
    error!("Device login backend error: {}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

/// Show a device login by its user code
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/oauth/device/{user_code}",
    params(("user_code" = String, Path, description = "The code scottyctl printed")),
    responses(
        (status = 200, description = "The device login", body = DeviceInfo),
        (status = 403, description = "Service accounts cannot approve device logins"),
        (status = 404, description = "Unknown, decided or expired code")
    ),
    security(("bearerAuth" = []))
)]
pub async fn get_device_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Path(user_code): Path<String>,
) -> Result<Json<DeviceInfo>, StatusCode> {
    let server = decider(&state, &user)?;
    let device = server
        .device(&user_code, &user.email)
        .await
        .map_err(backend_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(DeviceInfo {
        user_code: device.user_code,
        client_name: device.client_name,
        scope: device.scope,
    }))
}

/// Approve a device login
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/oauth/device/approve",
    request_body = DeviceDecision,
    responses(
        (status = 204, description = "Approved; scottyctl's next poll gets tokens"),
        (status = 403, description = "Service accounts cannot approve device logins"),
        (status = 404, description = "Nothing to approve")
    ),
    security(("bearerAuth" = []))
)]
pub async fn approve_device_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(decision): Json<DeviceDecision>,
) -> Result<StatusCode, StatusCode> {
    let server = decider(&state, &user)?;
    server
        .approve_device(&decision.user_code, &user.email)
        .await
        .map_err(backend_error)?
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or(StatusCode::NOT_FOUND)
}

/// Deny a device login
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/oauth/device/deny",
    request_body = DeviceDecision,
    responses(
        (status = 204, description = "Denied; scottyctl's next poll gets access_denied"),
        (status = 403, description = "Service accounts cannot deny device logins"),
        (status = 404, description = "Nothing to deny")
    ),
    security(("bearerAuth" = []))
)]
pub async fn deny_device_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Json(decision): Json<DeviceDecision>,
) -> Result<StatusCode, StatusCode> {
    let server = decider(&state, &user)?;
    server
        .deny_device(&decision.user_code, &user.email)
        .await
        .map_err(backend_error)?
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or(StatusCode::NOT_FOUND)
}
