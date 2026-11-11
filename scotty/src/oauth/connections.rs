//! The connections API behind the frontend `/connections` page (design D5):
//! the caller's active CLI sessions and MCP grants, and revoking them.
//! Browser sessions (`scotty-web`) are not connections. `AdminWrite` lists
//! and revokes everyone's. Session users only; refusals are never 401.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Extension, Json,
};
use chrono::{DateTime, Utc};
use grantry::Grant;
use scotty_core::authorization::Permission;
use serde::{Deserialize, Serialize};
use tracing::error;
use uuid::Uuid;

use super::server::WEB_CLIENT_ID;
use super::store::{active_grants, REALM_MCP};
use crate::api::basic_auth::CurrentUser;
use crate::app_state::SharedAppState;
use crate::services::authorization::decision::Target;

/// One active connection.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Connection {
    pub id: Uuid,
    /// `mcp` or `cli`.
    pub kind: String,
    pub client_id: String,
    pub client_name: String,
    /// The host of the redirect URI; none for CLI sessions.
    pub redirect_host: Option<String>,
    pub scope: String,
    /// The user's email.
    pub subject: String,
    pub created_at: DateTime<Utc>,
    /// The last refresh.
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ConnectionsQuery {
    /// Everyone's connections; needs `admin_write`.
    #[serde(default)]
    pub all: bool,
}

fn connection(realm: &str, grant: Grant<String>) -> Connection {
    Connection {
        id: grant.id,
        kind: if realm == REALM_MCP { "mcp" } else { "cli" }.into(),
        redirect_host: Some(grantry::redirect::host_of(&grant.redirect_uri))
            .filter(|h| !h.is_empty()),
        client_id: grant.client_id,
        client_name: grant.client_name,
        scope: grant.scope,
        subject: grant.subject,
        created_at: grant.created_at,
        last_used_at: grant.last_refreshed_at,
        expires_at: grant.expires_at,
    }
}

fn backend_error(e: impl std::fmt::Display) -> StatusCode {
    error!("Connections backend error: {}", e);
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn is_admin(state: &SharedAppState, user: &CurrentUser) -> bool {
    state
        .auth_service
        .decide(&user.email, Permission::AdminWrite.into(), Target::Global)
        .is_allowed()
}

/// The person's database, or 403/404.
fn db<'a>(
    state: &'a SharedAppState,
    user: &CurrentUser,
) -> Result<&'a sqlx::SqlitePool, StatusCode> {
    super::require_person(user)?;
    state.oauth_db.as_ref().ok_or(StatusCode::NOT_FOUND)
}

/// List active connections
#[utoipa::path(
    get,
    path = "/api/v1/authenticated/connections",
    params(ConnectionsQuery),
    responses(
        (status = 200, description = "Active connections, newest first", body = [Connection]),
        (status = 403, description = "Service account, or `all` without admin_write"),
        (status = 404, description = "Not in oauth mode")
    ),
    security(("bearerAuth" = []))
)]
pub async fn list_connections_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Query(query): Query<ConnectionsQuery>,
) -> Result<Json<Vec<Connection>>, StatusCode> {
    let pool = db(&state, &user)?;
    if query.all && !is_admin(&state, &user).await {
        return Err(StatusCode::FORBIDDEN);
    }
    let subject = (!query.all).then_some(user.email.as_str());
    let grants = active_grants(pool, subject, None)
        .await
        .map_err(backend_error)?;
    Ok(Json(
        grants
            .into_iter()
            .filter(|(_, g)| g.client_id != WEB_CLIENT_ID)
            .map(|(realm, g)| connection(&realm, g))
            .collect(),
    ))
}

/// Revoke a connection
#[utoipa::path(
    delete,
    path = "/api/v1/authenticated/connections/{id}",
    params(("id" = Uuid, Path, description = "The connection (grant) id")),
    responses(
        (status = 204, description = "Revoked; its refresh and access tokens stop working"),
        (status = 403, description = "Service accounts cannot revoke connections"),
        (status = 404, description = "No such active connection of the caller")
    ),
    security(("bearerAuth" = []))
)]
pub async fn revoke_connection_handler(
    State(state): State<SharedAppState>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let pool = db(&state, &user)?;
    let (realm, grant) = active_grants(pool, None, Some(id))
        .await
        .map_err(backend_error)?
        .into_iter()
        .next()
        .filter(|(_, g)| g.client_id != WEB_CLIENT_ID)
        .ok_or(StatusCode::NOT_FOUND)?;
    // Someone else's grant is "not found" unless the caller is an admin.
    if grant.subject != user.email && !is_admin(&state, &user).await {
        return Err(StatusCode::NOT_FOUND);
    }
    // Revoke through the grant's own server, so its Events fire.
    let server = if realm == REALM_MCP {
        state.mcp_server.as_ref()
    } else {
        state.session_server.as_ref()
    }
    .ok_or(StatusCode::NOT_FOUND)?;
    let ip = super::device::client_ip(&headers);
    server
        .revoke(&grant, "user_revoked", ip.as_deref())
        .await
        .map_err(backend_error)?;
    Ok(StatusCode::NO_CONTENT)
}
