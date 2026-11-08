//! The grantry session server (design D2): first-party sessions of
//! `scotty-web` and `scottyctl`, audience `scotty-api`, kind `session`.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use grantry::client::{DocumentPolicy, FetchError, Fetched, MetadataFetcher};
use grantry::store::{BackendError, Events, Grant};
use grantry::{Config, Recheck, ScopeSpec, Server, Signer, Subjects};
use sqlx::SqlitePool;
use url::Url;

use super::store::{SqliteStore, REALM_SESSION};
use crate::services::AuthorizationService;

pub const SESSION_RESOURCE: &str = "scotty-api";
pub const SESSION_KIND: &str = "session";
pub const WEB_CLIENT_ID: &str = "scotty-web";
const ACCESS_TOKEN_SECS: u64 = 12 * 3600;
const GRANT_LIFETIME_DAYS: i64 = 30;

/// The session server; a newtype only so `AppState` can stay `Debug`.
#[derive(Clone)]
pub struct SessionServer(pub Server<String>);

impl std::ops::Deref for SessionServer {
    type Target = Server<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for SessionServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionServer")
            .field("config", &self.0.config)
            .finish_non_exhaustive()
    }
}

/// Build the session server over the `session` realm of `pool`.
pub fn session_server(
    pool: SqlitePool,
    jwt_secret: &[u8],
    auth_service: Arc<AuthorizationService>,
) -> SessionServer {
    let config = Config {
        resource: SESSION_RESOURCE.into(),
        token_kind: SESSION_KIND.into(),
        scopes: ScopeSpec::new(&["api", "offline_access"], &["api"], "offline_access"),
        signer: Signer::hmac(jsonwebtoken::Algorithm::HS512, jwt_secret),
        access_token_secs: ACCESS_TOKEN_SECS,
        // No authorization code flow on this server; grantry still wants these.
        code_ttl: Duration::seconds(60),
        pending_ttl: Duration::minutes(10),
        grant_lifetime: Duration::days(GRANT_LIFETIME_DAYS),
        documents: DocumentPolicy::default(),
    };
    SessionServer(Server::new(
        config,
        Arc::new(SqliteStore::new(pool, REALM_SESSION)),
        Arc::new(CasbinSubjects(auth_service)),
        Arc::new(TracingEvents),
        Arc::new(NoFetch),
    ))
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

struct TracingEvents;

#[async_trait]
impl Events<String> for TracingEvents {
    async fn grant_created(&self, grant: &Grant<String>) {
        tracing::info!(
            grant_id = %grant.id,
            subject = %grant.subject,
            client_id = %grant.client_id,
            "session grant created"
        );
    }

    async fn grant_revoked(&self, grant: &Grant<String>, reason: &str, client_ip: Option<&str>) {
        tracing::info!(
            grant_id = %grant.id,
            subject = %grant.subject,
            client_id = %grant.client_id,
            reason,
            client_ip,
            "session grant revoked"
        );
    }
}

/// The session server only serves first-party clients and never resolves a
/// client metadata document.
struct NoFetch;

#[async_trait]
impl MetadataFetcher for NoFetch {
    async fn fetch(&self, _url: &Url, _limit: usize) -> Result<Fetched, FetchError> {
        Err(FetchError(
            "the session server resolves no documents".into(),
        ))
    }
}
