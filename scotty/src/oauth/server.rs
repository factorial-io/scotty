//! The grantry servers (design D2): the session server for first-party
//! sessions of `scotty-web` and `scottyctl` (audience `scotty-api`, kind
//! `session`), and the mcp server for MCP clients (audience `{base}/mcp`, kind
//! `mcp`). Both share one database (separate realms) and one signer.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use grantry::store::{BackendError, Events, Grant};
use grantry::{Algorithm, Config, NoFetch, Recheck, ScopeSpec, Server, Signer, Subjects};
use sqlx::SqlitePool;

use super::fetch::EgressFetcher;
use super::store::{SqliteStore, REALM_MCP, REALM_SESSION};
use crate::services::AuthorizationService;

pub const SESSION_RESOURCE: &str = "scotty-api";
pub const SESSION_KIND: &str = "session";
pub const WEB_CLIENT_ID: &str = "scotty-web";
pub const CLI_CLIENT_ID: &str = "scottyctl";
const ACCESS_TOKEN_SECS: u64 = 12 * 3600;
const GRANT_LIFETIME_DAYS: i64 = 30;

pub const MCP_KIND: &str = "mcp";
/// The MCP scopes (spec: Scopes narrow the user's permissions).
pub const MCP_SCOPES: [&str; 4] = ["read", "create", "destroy", "offline_access"];

/// The session server.
pub type SessionServer = Server<String>;
/// The MCP authorization server.
pub type McpServer = Server<String>;

/// The protected resource of the mcp server: `{base}/mcp`.
pub fn mcp_resource(base: &str) -> String {
    format!("{base}/mcp")
}

/// Build the session server over the `session` realm of `pool`.
pub fn session_server(
    pool: SqlitePool,
    jwt_secret: &[u8],
    auth_service: Arc<AuthorizationService>,
) -> SessionServer {
    let mut config = Config::new(
        SESSION_RESOURCE,
        SESSION_KIND,
        ScopeSpec::new(&["api", "offline_access"], &["api"], "offline_access"),
        Signer::hmac(Algorithm::HS512, jwt_secret),
    );
    config.access_token_secs = ACCESS_TOKEN_SECS;
    config.grant_lifetime = Duration::days(GRANT_LIFETIME_DAYS);
    Server::new(
        config,
        Arc::new(SqliteStore::new(pool, REALM_SESSION)),
        Arc::new(CasbinSubjects(auth_service)),
        Arc::new(TracingEvents(REALM_SESSION)),
        Arc::new(NoFetch),
    )
}

/// Build the mcp server over the `mcp` realm of `pool`, for the public base
/// URL `base`: 15-minute access tokens, 60-second codes (grantry defaults),
/// 30-day grants, metadata documents fetched under [`EgressFetcher`].
pub fn mcp_server(
    pool: SqlitePool,
    jwt_secret: &[u8],
    auth_service: Arc<AuthorizationService>,
    base: &str,
) -> McpServer {
    let mut config = Config::new(
        mcp_resource(base),
        MCP_KIND,
        ScopeSpec::new(&MCP_SCOPES, &["read", "create"], "offline_access"),
        Signer::hmac(Algorithm::HS512, jwt_secret),
    );
    config.grant_lifetime = Duration::days(GRANT_LIFETIME_DAYS);
    Server::new(
        config,
        Arc::new(SqliteStore::new(pool, REALM_MCP)),
        Arc::new(CasbinSubjects(auth_service)),
        Arc::new(TracingEvents(REALM_MCP)),
        Arc::new(EgressFetcher),
    )
}

/// A user keeps a session while Casbin gives them any permission in any scope.
struct CasbinSubjects(Arc<AuthorizationService>);

#[async_trait]
impl Subjects<String> for CasbinSubjects {
    async fn recheck(&self, grant: &Grant<String>) -> Result<Recheck, BackendError> {
        let permissions = self.0.get_user_permissions(&grant.subject).await;
        if permissions.values().all(Vec::is_empty) {
            return Ok(Recheck::Revoke("no_role".into()));
        }
        let mut claims = serde_json::Map::new();
        claims.insert("sub".into(), grant.subject.clone().into());
        Ok(Recheck::Allow(claims))
    }
}

/// Audit log of grant events, tagged with the realm.
struct TracingEvents(&'static str);

#[async_trait]
impl Events<String> for TracingEvents {
    async fn grant_created(&self, grant: &Grant<String>) {
        tracing::info!(
            grant_id = %grant.id,
            subject = %grant.subject,
            client_id = %grant.client_id,
            realm = self.0,
            "grant created"
        );
    }

    async fn grant_revoked(&self, grant: &Grant<String>, reason: &str, client_ip: Option<&str>) {
        tracing::info!(
            grant_id = %grant.id,
            subject = %grant.subject,
            client_id = %grant.client_id,
            reason,
            client_ip,
            realm = self.0,
            "grant revoked"
        );
    }
}
