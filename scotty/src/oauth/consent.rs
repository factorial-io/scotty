//! The consent endpoints behind the frontend `/oauth/consent/{id}` page
//! (design D5): show a pending MCP authorization, approve it with the kept
//! scopes, or deny it. Session users only. Refusals are never 401, which the
//! frontend reads as an expired session.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use grantry::server::ApproveError;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use super::server::McpServer;
use crate::api::basic_auth::CurrentUser;
use crate::app_state::SharedAppState;

/// A pending authorization as the consent page shows it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConsentInfo {
    pub id: Uuid,
    pub client_id: String,
    /// Self-declared by the client; show it after `redirect_host`.
    pub client_name: String,
    /// Where the authorization goes; show it first.
    pub redirect_host: String,
    /// Every redirect URI of the client is a loopback address.
    pub loopback_only: bool,
    pub scopes: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ConsentApproval {
    /// A non-empty subset of the requested scopes.
    pub scopes: Vec<String>,
}

/// Where the browser goes next: the client's redirect URI with the code, or
/// with `error=access_denied`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConsentRedirect {
    pub redirect: String,
}

fn decider<'a>(state: &'a SharedAppState, user: &CurrentUser) -> Result<&'a McpServer, StatusCode> {
    super::require_person(user)?;
    state.mcp_server.as_ref().ok_or(StatusCode::NOT_FOUND)
}

fn backend_error(e: impl std::fmt::Display) -> StatusCode {
    error!("Consent backend error: {}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

/// Show a pending MCP authorization
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/oauth/consent/{id}",
    params(("id" = Uuid, Path, description = "The pending authorization")),
    responses(
        (status = 200, description = "The pending authorization", body = ConsentInfo),
        (status = 403, description = "Service accounts cannot consent"),
        (status = 404, description = "Unknown, decided, expired or another user's")
    ),
    security(("bearerAuth" = []))
)]
pub async fn get_consent_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConsentInfo>, StatusCode> {
    let server = decider(&state, &user)?;
    let pending = server
        .pending(id, &user.email)
        .await
        .map_err(backend_error)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(ConsentInfo {
        id: pending.id,
        redirect_host: grantry::redirect::host_of(&pending.redirect_uri),
        scopes: pending
            .scope
            .split_whitespace()
            .map(str::to_owned)
            .collect(),
        client_id: pending.client_id,
        client_name: pending.client_name,
        loopback_only: pending.loopback_only,
    }))
}

/// Approve a pending MCP authorization
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/oauth/consent/{id}/approve",
    params(("id" = Uuid, Path, description = "The pending authorization")),
    request_body = ConsentApproval,
    responses(
        (status = 200, description = "Approved; send the browser to `redirect`", body = ConsentRedirect),
        (status = 400, description = "Scopes empty or not a subset of the requested ones"),
        (status = 403, description = "Service accounts cannot consent"),
        (status = 404, description = "Nothing to approve")
    ),
    security(("bearerAuth" = []))
)]
pub async fn approve_consent_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Json(approval): Json<ConsentApproval>,
) -> Result<Json<ConsentRedirect>, StatusCode> {
    let server = decider(&state, &user)?;
    match server
        .approve(id, &user.email, Some(&approval.scopes))
        .await
    {
        Ok(approved) => Ok(Json(ConsentRedirect {
            redirect: approved.redirect,
        })),
        Err(ApproveError::NotFound) => Err(StatusCode::NOT_FOUND),
        Err(ApproveError::InvalidScope) => Err(StatusCode::BAD_REQUEST),
        Err(ApproveError::Backend(e)) => Err(backend_error(e)),
    }
}

/// Deny a pending MCP authorization
#[utoipa::path(
    post,
    path = "/api/v1/authenticated/oauth/consent/{id}/deny",
    params(("id" = Uuid, Path, description = "The pending authorization")),
    responses(
        (status = 200, description = "Denied; send the browser to `redirect`", body = ConsentRedirect),
        (status = 403, description = "Service accounts cannot consent"),
        (status = 404, description = "Nothing to deny")
    ),
    security(("bearerAuth" = []))
)]
pub async fn deny_consent_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConsentRedirect>, StatusCode> {
    let server = decider(&state, &user)?;
    server
        .deny(id, &user.email)
        .await
        .map_err(backend_error)?
        .map(|denied| {
            Json(ConsentRedirect {
                redirect: denied.redirect,
            })
        })
        .ok_or(StatusCode::NOT_FOUND)
}
