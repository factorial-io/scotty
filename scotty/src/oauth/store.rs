//! [`grantry::OAuthStore`] over SQLite (design D1, D2).
//!
//! One database holds both grantry servers. Pending authorizations, grants,
//! codes, refresh tokens and device authorizations carry a `realm` and every
//! lookup filters by it, so a row of the `session` store is invisible to the
//! `mcp` store and the reverse. Clients and cached metadata documents are
//! shared. Timestamps are unix milliseconds, ids TEXT UUIDs.
//!
//! Every *atomic* trait method is one conditional statement judged by its
//! affected rows, or a transaction whose first statement is that conditional
//! write (so it holds SQLite's write lock before it decides anything).

use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Duration, SubsecRound, Utc};
use grantry::device::{Device, DeviceStatus};
use grantry::store::{
    BackendError, Code, Grant, NewDevice, NewGrant, NewPending, OAuthStore, Pending, RefreshToken,
    RegisteredClient,
};
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// The realm of first-party sessions (`scotty-web`, `scottyctl`).
pub const REALM_SESSION: &str = "session";
/// The realm of MCP clients.
#[allow(dead_code)] // used by the mcp server (task 4.1)
pub const REALM_MCP: &str = "mcp";

/// How long a registered client may go without a grant before `sweep`
/// removes it (as grantry's `MemoryStore`).
const UNUSED_CLIENT_DAYS: i64 = 7;
/// How long an expired or revoked grant is kept (for the connections list)
/// before `sweep` removes it with its codes and refresh tokens.
const DEAD_GRANT_DAYS: i64 = 30;

/// Open (creating it if missing) the database at `path` in WAL mode with
/// foreign keys on, and run the embedded migrations.
pub async fn open(path: impl AsRef<Path>) -> anyhow::Result<SqlitePool> {
    let path = path.as_ref();
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// See the module docs. The subject is the user's email.
#[derive(Debug, Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
    realm: &'static str,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool, realm: &'static str) -> Self {
        Self { pool, realm }
    }

    #[allow(dead_code)] // used by the connections API (task 4.4)
    /// The active grants of this realm, newest first; all subjects for `None`.
    pub async fn list_grants(
        &self,
        subject: Option<&str>,
    ) -> Result<Vec<Grant<String>>, BackendError> {
        let rows = sqlx::query(
            "SELECT * FROM oauth_grants
             WHERE realm = ? AND revoked_at IS NULL AND expires_at > ?
               AND (? IS NULL OR subject = ?)
             ORDER BY created_at DESC",
        )
        .bind(self.realm)
        .bind(ms(now()))
        .bind(subject)
        .bind(subject)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(grant_from).collect()
    }

    async fn insert_grant(
        &self,
        conn: &mut sqlx::SqliteConnection,
        grant: &Grant<String>,
    ) -> Result<(), BackendError> {
        sqlx::query(
            "INSERT INTO oauth_grants
             (id, realm, subject, client_id, client_name, redirect_uri, scope, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(grant.id.to_string())
        .bind(self.realm)
        .bind(&grant.subject)
        .bind(&grant.client_id)
        .bind(&grant.client_name)
        .bind(&grant.redirect_uri)
        .bind(&grant.scope)
        .bind(ms(grant.created_at))
        .bind(ms(grant.expires_at))
        .execute(conn)
        .await?;
        Ok(())
    }
}

/// The current time at the stored precision, so a row read back equals the
/// struct a write returned.
fn now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(3)
}

fn ms(at: DateTime<Utc>) -> i64 {
    at.timestamp_millis()
}

fn at(row: &SqliteRow, col: &str) -> Result<DateTime<Utc>, BackendError> {
    let ms: i64 = row.try_get(col)?;
    DateTime::from_timestamp_millis(ms).ok_or_else(|| format!("bad timestamp in {col}").into())
}

fn at_opt(row: &SqliteRow, col: &str) -> Result<Option<DateTime<Utc>>, BackendError> {
    let ms: Option<i64> = row.try_get(col)?;
    ms.map(|ms| {
        DateTime::from_timestamp_millis(ms).ok_or_else(|| format!("bad timestamp in {col}").into())
    })
    .transpose()
}

fn uuid(row: &SqliteRow, col: &str) -> Result<Uuid, BackendError> {
    Ok(Uuid::parse_str(row.try_get::<&str, _>(col)?)?)
}

fn new_grant(new: NewGrant<String>, now: DateTime<Utc>) -> Grant<String> {
    Grant {
        id: Uuid::new_v4(),
        subject: new.subject,
        client_id: new.client_id,
        client_name: new.client_name,
        redirect_uri: String::new(),
        scope: new.scope,
        created_at: now,
        expires_at: now + new.lifetime,
        last_refreshed_at: None,
        revoked_at: None,
        revoke_reason: None,
    }
}

fn client_from(row: &SqliteRow) -> Result<RegisteredClient, BackendError> {
    Ok(RegisteredClient {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        redirect_uris: serde_json::from_str(row.try_get::<&str, _>("redirect_uris")?)?,
        created_at: at(row, "created_at")?,
    })
}

fn pending_from(row: &SqliteRow) -> Result<Pending<String>, BackendError> {
    Ok(Pending {
        id: uuid(row, "id")?,
        client_id: row.try_get("client_id")?,
        client_name: row.try_get("client_name")?,
        loopback_only: row.try_get("loopback_only")?,
        redirect_uri: row.try_get("redirect_uri")?,
        scope: row.try_get("scope")?,
        state: row.try_get("state")?,
        code_challenge: row.try_get("code_challenge")?,
        resource: row.try_get("resource")?,
        subject: row.try_get("subject")?,
        created_at: at(row, "created_at")?,
        expires_at: at(row, "expires_at")?,
    })
}

fn grant_from(row: &SqliteRow) -> Result<Grant<String>, BackendError> {
    Ok(Grant {
        id: uuid(row, "id")?,
        subject: row.try_get("subject")?,
        client_id: row.try_get("client_id")?,
        client_name: row.try_get("client_name")?,
        redirect_uri: row.try_get("redirect_uri")?,
        scope: row.try_get("scope")?,
        created_at: at(row, "created_at")?,
        expires_at: at(row, "expires_at")?,
        last_refreshed_at: at_opt(row, "last_refreshed_at")?,
        revoked_at: at_opt(row, "revoked_at")?,
        revoke_reason: row.try_get("revoke_reason")?,
    })
}

fn code_from(row: &SqliteRow) -> Result<Code, BackendError> {
    Ok(Code {
        id: uuid(row, "id")?,
        grant_id: uuid(row, "grant_id")?,
        client_id: row.try_get("client_id")?,
        redirect_uri: row.try_get("redirect_uri")?,
        resource: row.try_get("resource")?,
        code_challenge: row.try_get("code_challenge")?,
        expires_at: at(row, "expires_at")?,
        used_at: at_opt(row, "used_at")?,
    })
}

fn refresh_from(row: &SqliteRow) -> Result<RefreshToken, BackendError> {
    Ok(RefreshToken {
        id: uuid(row, "id")?,
        grant_id: uuid(row, "grant_id")?,
        issued_at: at(row, "issued_at")?,
        rotated_at: at_opt(row, "rotated_at")?,
    })
}

fn device_from(row: &SqliteRow) -> Result<Device<String>, BackendError> {
    let status: String = row.try_get("status")?;
    let interval: i64 = row.try_get("interval_seconds")?;
    Ok(Device {
        id: uuid(row, "id")?,
        client_id: row.try_get("client_id")?,
        client_name: row.try_get("client_name")?,
        scope: row.try_get("scope")?,
        user_code: row.try_get("user_code")?,
        status: status.parse()?,
        interval_seconds: u32::try_from(interval)?,
        last_polled_at: at_opt(row, "last_polled_at")?,
        subject: row.try_get("subject")?,
        created_at: at(row, "created_at")?,
        expires_at: at(row, "expires_at")?,
    })
}

#[async_trait]
impl OAuthStore for SqliteStore {
    type Subject = String;

    async fn insert_client(
        &self,
        id: String,
        name: String,
        redirect_uris: &[String],
    ) -> Result<RegisteredClient, BackendError> {
        let client = RegisteredClient {
            id,
            name,
            redirect_uris: redirect_uris.to_vec(),
            created_at: now(),
        };
        sqlx::query(
            "INSERT INTO oauth_clients (id, name, redirect_uris, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(&client.id)
        .bind(&client.name)
        .bind(serde_json::to_string(&client.redirect_uris)?)
        .bind(ms(client.created_at))
        .execute(&self.pool)
        .await?;
        Ok(client)
    }

    async fn use_client(&self, id: &str) -> Result<Option<RegisteredClient>, BackendError> {
        sqlx::query("UPDATE oauth_clients SET last_used_at = ? WHERE id = ? RETURNING *")
            .bind(ms(now()))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(client_from)
            .transpose()
    }

    async fn cached_document(&self, url: &str) -> Result<Option<Value>, BackendError> {
        let doc: Option<String> = sqlx::query_scalar(
            "SELECT document FROM oauth_documents WHERE url = ? AND expires_at > ?",
        )
        .bind(url)
        .bind(ms(now()))
        .fetch_optional(&self.pool)
        .await?;
        Ok(doc.map(|d| serde_json::from_str(&d)).transpose()?)
    }

    async fn store_document(
        &self,
        url: &str,
        document: Value,
        expires_at: DateTime<Utc>,
    ) -> Result<(), BackendError> {
        sqlx::query(
            "INSERT INTO oauth_documents (url, document, expires_at) VALUES (?, ?, ?)
             ON CONFLICT (url) DO UPDATE SET document = excluded.document, expires_at = excluded.expires_at",
        )
        .bind(url)
        .bind(document.to_string())
        .bind(ms(expires_at))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_pending(&self, new: NewPending) -> Result<Pending<String>, BackendError> {
        let now = now();
        let pending = Pending {
            id: Uuid::new_v4(),
            client_id: new.client_id,
            client_name: new.client_name,
            loopback_only: new.loopback_only,
            redirect_uri: new.redirect_uri,
            scope: new.scope,
            state: new.state,
            code_challenge: new.code_challenge,
            resource: new.resource,
            subject: None,
            created_at: now,
            expires_at: now + new.ttl,
        };
        sqlx::query(
            "INSERT INTO oauth_pending
             (id, realm, client_id, client_name, loopback_only, redirect_uri, scope, state,
              code_challenge, resource, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(pending.id.to_string())
        .bind(self.realm)
        .bind(&pending.client_id)
        .bind(&pending.client_name)
        .bind(pending.loopback_only)
        .bind(&pending.redirect_uri)
        .bind(&pending.scope)
        .bind(&pending.state)
        .bind(&pending.code_challenge)
        .bind(&pending.resource)
        .bind(ms(pending.created_at))
        .bind(ms(pending.expires_at))
        .execute(&self.pool)
        .await?;
        Ok(pending)
    }

    async fn claim_pending(
        &self,
        id: Uuid,
        subject: &String,
    ) -> Result<Option<Pending<String>>, BackendError> {
        // Binds an unbound row and returns it only to its subject, in one
        // statement.
        sqlx::query(
            "UPDATE oauth_pending SET subject = COALESCE(subject, ?1)
             WHERE id = ?2 AND realm = ?3 AND expires_at > ?4
               AND (subject IS NULL OR subject = ?1)
             RETURNING *",
        )
        .bind(subject)
        .bind(id.to_string())
        .bind(self.realm)
        .bind(ms(now()))
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(pending_from)
        .transpose()
    }

    async fn take_pending(
        &self,
        id: Uuid,
        subject: &String,
    ) -> Result<Option<Pending<String>>, BackendError> {
        let row = sqlx::query(
            "DELETE FROM oauth_pending
             WHERE id = ?1 AND realm = ?2 AND expires_at > ?3
               AND (subject IS NULL OR subject = ?4)
             RETURNING *",
        )
        .bind(id.to_string())
        .bind(self.realm)
        .bind(ms(now()))
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let mut pending = pending_from(&row)?;
        pending.subject = Some(subject.clone());
        Ok(Some(pending))
    }

    async fn create_grant_with_code(
        &self,
        approved: &Pending<String>,
        subject: &String,
        lifetime: Duration,
        code_hash: String,
        code_ttl: Duration,
    ) -> Result<Grant<String>, BackendError> {
        let now = now();
        let grant = Grant {
            redirect_uri: approved.redirect_uri.clone(),
            ..new_grant(
                NewGrant {
                    subject: subject.clone(),
                    client_id: approved.client_id.clone(),
                    client_name: approved.client_name.clone(),
                    scope: approved.scope.clone(),
                    lifetime,
                },
                now,
            )
        };
        let mut tx = self.pool.begin().await?;
        self.insert_grant(&mut tx, &grant).await?;
        sqlx::query(
            "INSERT INTO oauth_codes
             (id, realm, code_hash, grant_id, client_id, redirect_uri, resource, code_challenge, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(self.realm)
        .bind(code_hash)
        .bind(grant.id.to_string())
        .bind(&approved.client_id)
        .bind(&approved.redirect_uri)
        .bind(&approved.resource)
        .bind(&approved.code_challenge)
        .bind(ms(now + code_ttl))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(grant)
    }

    async fn find_code(&self, code_hash: &str) -> Result<Option<Code>, BackendError> {
        sqlx::query("SELECT * FROM oauth_codes WHERE code_hash = ? AND realm = ?")
            .bind(code_hash)
            .bind(self.realm)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(code_from)
            .transpose()
    }

    async fn use_code(&self, id: Uuid) -> Result<bool, BackendError> {
        let done = sqlx::query(
            "UPDATE oauth_codes SET used_at = ? WHERE id = ? AND realm = ? AND used_at IS NULL",
        )
        .bind(ms(now()))
        .bind(id.to_string())
        .bind(self.realm)
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    async fn find_grant(&self, id: Uuid) -> Result<Option<Grant<String>>, BackendError> {
        sqlx::query("SELECT * FROM oauth_grants WHERE id = ? AND realm = ?")
            .bind(id.to_string())
            .bind(self.realm)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(grant_from)
            .transpose()
    }

    async fn create_grant(&self, new: NewGrant<String>) -> Result<Grant<String>, BackendError> {
        let grant = new_grant(new, now());
        let mut conn = self.pool.acquire().await?;
        self.insert_grant(&mut conn, &grant).await?;
        Ok(grant)
    }

    async fn insert_device(&self, new: NewDevice) -> Result<Option<Device<String>>, BackendError> {
        let now = now();
        let device = Device {
            id: Uuid::new_v4(),
            client_id: new.client_id,
            client_name: new.client_name,
            scope: new.scope,
            user_code: new.user_code,
            status: DeviceStatus::Pending,
            interval_seconds: new.interval_seconds,
            last_polled_at: None,
            subject: None,
            created_at: now,
            expires_at: now + new.ttl,
        };
        // One statement: inserts only while no live row of this realm holds
        // the user code.
        let done = sqlx::query(
            "INSERT INTO oauth_devices
             (id, realm, device_code_hash, client_id, client_name, scope, user_code, status,
              interval_seconds, created_at, expires_at)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11
             WHERE NOT EXISTS (
                 SELECT 1 FROM oauth_devices WHERE realm = ?2 AND user_code = ?7 AND expires_at > ?10
             )",
        )
        .bind(device.id.to_string())
        .bind(self.realm)
        .bind(new.device_code_hash)
        .bind(&device.client_id)
        .bind(&device.client_name)
        .bind(&device.scope)
        .bind(&device.user_code)
        .bind(device.status.as_str())
        .bind(i64::from(device.interval_seconds))
        .bind(ms(device.created_at))
        .bind(ms(device.expires_at))
        .execute(&self.pool)
        .await?;
        Ok((done.rows_affected() == 1).then_some(device))
    }

    async fn find_device(
        &self,
        device_code_hash: &str,
    ) -> Result<Option<Device<String>>, BackendError> {
        sqlx::query("SELECT * FROM oauth_devices WHERE device_code_hash = ? AND realm = ?")
            .bind(device_code_hash)
            .bind(self.realm)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(device_from)
            .transpose()
    }

    async fn device_by_user_code(
        &self,
        user_code: &str,
        _subject: &String,
    ) -> Result<Option<Device<String>>, BackendError> {
        sqlx::query(
            "SELECT * FROM oauth_devices WHERE realm = ? AND user_code = ? AND expires_at > ? LIMIT 1",
        )
        .bind(self.realm)
        .bind(user_code)
        .bind(ms(now()))
        .fetch_optional(&self.pool)
        .await?
        .as_ref()
        .map(device_from)
        .transpose()
    }

    async fn decide_device(
        &self,
        id: Uuid,
        subject: &String,
        approve: bool,
    ) -> Result<bool, BackendError> {
        let status = if approve {
            DeviceStatus::Approved
        } else {
            DeviceStatus::Denied
        };
        let done = sqlx::query(
            "UPDATE oauth_devices SET status = ?, subject = ?
             WHERE id = ? AND realm = ? AND status = 'pending' AND expires_at > ?",
        )
        .bind(status.as_str())
        .bind(subject)
        .bind(id.to_string())
        .bind(self.realm)
        .bind(ms(now()))
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    async fn record_poll(
        &self,
        id: Uuid,
        at: DateTime<Utc>,
        interval_seconds: u32,
    ) -> Result<(), BackendError> {
        sqlx::query(
            "UPDATE oauth_devices SET last_polled_at = ?, interval_seconds = ? WHERE id = ? AND realm = ?",
        )
        .bind(ms(at))
        .bind(i64::from(interval_seconds))
        .bind(id.to_string())
        .bind(self.realm)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn redeem_device(
        &self,
        id: Uuid,
        grant: NewGrant<String>,
    ) -> Result<Option<Grant<String>>, BackendError> {
        let now = now();
        let mut tx = self.pool.begin().await?;
        let done = sqlx::query(
            "UPDATE oauth_devices SET status = 'redeemed'
             WHERE id = ? AND realm = ? AND status = 'approved' AND expires_at > ?",
        )
        .bind(id.to_string())
        .bind(self.realm)
        .bind(ms(now))
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() != 1 {
            return Ok(None); // dropping `tx` rolls back
        }
        let grant = new_grant(grant, now);
        self.insert_grant(&mut tx, &grant).await?;
        tx.commit().await?;
        Ok(Some(grant))
    }

    async fn revoke_grant(&self, id: Uuid, reason: &str) -> Result<bool, BackendError> {
        let done = sqlx::query(
            "UPDATE oauth_grants SET revoked_at = ?, revoke_reason = ?
             WHERE id = ? AND realm = ? AND revoked_at IS NULL",
        )
        .bind(ms(now()))
        .bind(reason)
        .bind(id.to_string())
        .bind(self.realm)
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    async fn insert_refresh_token(
        &self,
        grant: &Grant<String>,
        token_hash: String,
    ) -> Result<(), BackendError> {
        sqlx::query(
            "INSERT INTO oauth_refresh_tokens (id, realm, token_hash, grant_id, issued_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(self.realm)
        .bind(token_hash)
        .bind(grant.id.to_string())
        .bind(ms(now()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_refresh_token(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, BackendError> {
        sqlx::query("SELECT * FROM oauth_refresh_tokens WHERE token_hash = ? AND realm = ?")
            .bind(token_hash)
            .bind(self.realm)
            .fetch_optional(&self.pool)
            .await?
            .as_ref()
            .map(refresh_from)
            .transpose()
    }

    async fn rotate_refresh_token(
        &self,
        old: &RefreshToken,
        grant: &Grant<String>,
        new_hash: String,
    ) -> Result<bool, BackendError> {
        let now = ms(now());
        let mut tx = self.pool.begin().await?;
        let done = sqlx::query(
            "UPDATE oauth_refresh_tokens SET rotated_at = ?
             WHERE id = ? AND realm = ? AND rotated_at IS NULL",
        )
        .bind(now)
        .bind(old.id.to_string())
        .bind(self.realm)
        .execute(&mut *tx)
        .await?;
        if done.rows_affected() != 1 {
            return Ok(false);
        }
        sqlx::query(
            "INSERT INTO oauth_refresh_tokens (id, realm, token_hash, grant_id, issued_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(self.realm)
        .bind(new_hash)
        .bind(grant.id.to_string())
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE oauth_grants SET last_refreshed_at = ? WHERE id = ? AND realm = ?")
            .bind(now)
            .bind(grant.id.to_string())
            .bind(self.realm)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    /// Housekeeping by expiry only, so it ignores the realm: either store
    /// sweeps the whole database. Besides the trait's rows it removes grants
    /// expired or revoked more than [`DEAD_GRANT_DAYS`] ago, with their codes
    /// and refresh tokens (cascade); those are not counted.
    async fn sweep(&self) -> Result<u64, BackendError> {
        let now = now();
        let now_ms = ms(now);
        let dead = ms(now - Duration::days(DEAD_GRANT_DAYS));
        let unused = ms(now - Duration::days(UNUSED_CLIENT_DAYS));
        let mut removed = 0;
        for (sql, cutoff) in [
            ("DELETE FROM oauth_pending WHERE expires_at <= ?", now_ms),
            ("DELETE FROM oauth_devices WHERE expires_at <= ?", now_ms),
            ("DELETE FROM oauth_codes WHERE expires_at <= ?", now_ms),
            ("DELETE FROM oauth_documents WHERE expires_at <= ?", now_ms),
            (
                "DELETE FROM oauth_grants WHERE expires_at < ?1 OR revoked_at < ?1",
                dead,
            ),
            (
                "DELETE FROM oauth_clients WHERE created_at < ?
                 AND NOT EXISTS (SELECT 1 FROM oauth_grants g WHERE g.client_id = oauth_clients.id)",
                unused,
            ),
        ] {
            removed += sqlx::query(sql)
                .bind(cutoff)
                .execute(&self.pool)
                .await?
                .rows_affected();
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use grantry::client::{DocumentPolicy, FetchError, Fetched, MetadataFetcher};
    use grantry::server::{PollError, TokenError};
    use grantry::store::NoEvents;
    use grantry::{Config, Recheck, ScopeSpec, Server, Signer, Subjects};
    use serde_json::{json, Map};
    use sha2::{Digest, Sha256};
    use url::Url;

    use super::*;

    const RESOURCE: &str = "https://scotty.example/mcp";
    const CALLBACK: &str = "https://claude.ai/api/mcp/auth_callback";
    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk-and-some-more";

    async fn pool() -> (tempfile::TempDir, SqlitePool) {
        let dir = tempfile::tempdir().unwrap();
        let pool = open(dir.path().join("nested/scotty.sqlite")).await.unwrap();
        (dir, pool)
    }

    fn alice() -> String {
        "alice@example.com".into()
    }

    fn new_pending() -> NewPending {
        NewPending {
            client_id: "c".into(),
            client_name: "Client".into(),
            loopback_only: false,
            redirect_uri: CALLBACK.into(),
            scope: "read".into(),
            state: Some("xyz".into()),
            code_challenge: "challenge".into(),
            resource: RESOURCE.into(),
            ttl: Duration::minutes(10),
        }
    }

    fn new_grant_for(subject: String) -> NewGrant<String> {
        NewGrant {
            subject,
            client_id: "scottyctl".into(),
            client_name: "scottyctl on host".into(),
            scope: "read offline_access".into(),
            lifetime: Duration::days(30),
        }
    }

    fn new_device(hash: &str, user_code: &str) -> NewDevice {
        NewDevice {
            client_id: "scottyctl".into(),
            client_name: "scottyctl on host".into(),
            scope: "read offline_access".into(),
            device_code_hash: hash.into(),
            user_code: user_code.into(),
            interval_seconds: 5,
            ttl: Duration::minutes(10),
        }
    }

    /// Move `col` of every row of `table` `by` into the past.
    async fn age(pool: &SqlitePool, table: &str, col: &str, by: Duration) {
        sqlx::query(&format!("UPDATE {table} SET {col} = {col} - ?"))
            .bind(by.num_milliseconds())
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn pending_claims_and_takes_once() {
        let (_dir, pool) = pool().await;
        let store = SqliteStore::new(pool, REALM_MCP);
        let bob = "bob@example.com".to_string();
        let p = store.insert_pending(new_pending()).await.unwrap();

        let claimed = store.claim_pending(p.id, &alice()).await.unwrap().unwrap();
        assert_eq!(claimed.subject, Some(alice()));
        assert_eq!(claimed.state.as_deref(), Some("xyz"));
        assert!(store.claim_pending(p.id, &alice()).await.unwrap().is_some());
        assert!(
            store.claim_pending(p.id, &bob).await.unwrap().is_none(),
            "bound once"
        );
        assert!(store.take_pending(p.id, &bob).await.unwrap().is_none());

        let taken = store.take_pending(p.id, &alice()).await.unwrap().unwrap();
        assert_eq!(taken, claimed);
        assert!(
            store.take_pending(p.id, &alice()).await.unwrap().is_none(),
            "single use"
        );
        assert!(store.claim_pending(p.id, &alice()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn codes_and_refresh_tokens_are_used_once() {
        let (_dir, pool) = pool().await;
        let store = SqliteStore::new(pool, REALM_MCP);
        let p = store.insert_pending(new_pending()).await.unwrap();
        let grant = store
            .create_grant_with_code(
                &p,
                &alice(),
                Duration::days(30),
                "code-h".into(),
                Duration::minutes(1),
            )
            .await
            .unwrap();
        assert_eq!(
            store.find_grant(grant.id).await.unwrap().as_ref(),
            Some(&grant)
        );

        let code = store.find_code("code-h").await.unwrap().unwrap();
        assert_eq!(code.grant_id, grant.id);
        assert!(store.use_code(code.id).await.unwrap());
        assert!(!store.use_code(code.id).await.unwrap());
        assert!(store
            .find_code("code-h")
            .await
            .unwrap()
            .unwrap()
            .used_at
            .is_some());

        store
            .insert_refresh_token(&grant, "rt-1".into())
            .await
            .unwrap();
        let first = store.find_refresh_token("rt-1").await.unwrap().unwrap();
        assert!(store
            .rotate_refresh_token(&first, &grant, "rt-2".into())
            .await
            .unwrap());
        assert!(!store
            .rotate_refresh_token(&first, &grant, "rt-3".into())
            .await
            .unwrap());
        assert!(
            store.find_refresh_token("rt-3").await.unwrap().is_none(),
            "nothing left behind"
        );
        assert!(store
            .find_refresh_token("rt-1")
            .await
            .unwrap()
            .unwrap()
            .rotated_at
            .is_some());
        assert!(store
            .find_refresh_token("rt-2")
            .await
            .unwrap()
            .unwrap()
            .rotated_at
            .is_none());
        assert!(store
            .find_grant(grant.id)
            .await
            .unwrap()
            .unwrap()
            .last_refreshed_at
            .is_some());

        assert!(store.revoke_grant(grant.id, "logout").await.unwrap());
        assert!(!store.revoke_grant(grant.id, "logout").await.unwrap());
        assert!(store.list_grants(None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn devices_are_decided_and_redeemed_once() {
        let (_dir, pool) = pool().await;
        let store = SqliteStore::new(pool, REALM_SESSION);
        let device = store
            .insert_device(new_device("d1", "WDJB-MJHT"))
            .await
            .unwrap()
            .unwrap();
        assert!(
            store
                .insert_device(new_device("d2", "WDJB-MJHT"))
                .await
                .unwrap()
                .is_none(),
            "a live user code is taken"
        );
        let found = store
            .device_by_user_code("WDJB-MJHT", &alice())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found, device);

        assert!(
            store
                .redeem_device(device.id, new_grant_for(alice()))
                .await
                .unwrap()
                .is_none(),
            "not approved"
        );
        assert!(store
            .decide_device(device.id, &alice(), true)
            .await
            .unwrap());
        assert!(!store
            .decide_device(device.id, &alice(), false)
            .await
            .unwrap());

        let at = Utc::now();
        store.record_poll(device.id, at, 10).await.unwrap();
        let polled = store.find_device("d1").await.unwrap().unwrap();
        assert_eq!(polled.status, DeviceStatus::Approved);
        assert_eq!(polled.subject, Some(alice()));
        assert_eq!(polled.interval_seconds, 10);
        assert_eq!(polled.last_polled_at.map(ms), Some(ms(at)));

        let grant = store
            .redeem_device(device.id, new_grant_for(alice()))
            .await
            .unwrap()
            .unwrap();
        assert!(store
            .redeem_device(device.id, new_grant_for(alice()))
            .await
            .unwrap()
            .is_none());
        assert_eq!(
            store.find_device("d1").await.unwrap().unwrap().status,
            DeviceStatus::Redeemed
        );
        assert_eq!(
            store.list_grants(Some(&alice())).await.unwrap(),
            vec![grant]
        );
        assert!(store
            .list_grants(Some("bob@example.com"))
            .await
            .unwrap()
            .is_empty());

        // An expired code frees the user code.
        age(
            &store.pool,
            "oauth_devices",
            "expires_at",
            Duration::minutes(11),
        )
        .await;
        assert!(store
            .device_by_user_code("WDJB-MJHT", &alice())
            .await
            .unwrap()
            .is_none());
        assert!(store
            .insert_device(new_device("d3", "WDJB-MJHT"))
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn realms_do_not_see_each_other() {
        let (_dir, pool) = pool().await;
        let session = SqliteStore::new(pool.clone(), REALM_SESSION);
        let mcp = SqliteStore::new(pool, REALM_MCP);

        let p = session.insert_pending(new_pending()).await.unwrap();
        assert!(mcp.claim_pending(p.id, &alice()).await.unwrap().is_none());
        assert!(mcp.take_pending(p.id, &alice()).await.unwrap().is_none());
        let grant = session
            .create_grant_with_code(
                &p,
                &alice(),
                Duration::days(30),
                "code-h".into(),
                Duration::minutes(1),
            )
            .await
            .unwrap();
        session
            .insert_refresh_token(&grant, "rt".into())
            .await
            .unwrap();
        let device = session
            .insert_device(new_device("d", "BCDF-GHJK"))
            .await
            .unwrap()
            .unwrap();

        assert!(mcp.find_grant(grant.id).await.unwrap().is_none());
        assert!(!mcp.revoke_grant(grant.id, "x").await.unwrap());
        assert!(mcp.find_code("code-h").await.unwrap().is_none());
        assert!(mcp.find_refresh_token("rt").await.unwrap().is_none());
        assert!(mcp.list_grants(None).await.unwrap().is_empty());
        assert!(mcp.find_device("d").await.unwrap().is_none());
        assert!(mcp
            .device_by_user_code("BCDF-GHJK", &alice())
            .await
            .unwrap()
            .is_none());
        assert!(!mcp.decide_device(device.id, &alice(), true).await.unwrap());

        // The same user code is free in the other realm.
        assert!(mcp
            .insert_device(new_device("d-mcp", "BCDF-GHJK"))
            .await
            .unwrap()
            .is_some());
        assert!(session.find_grant(grant.id).await.unwrap().is_some());

        // Clients are shared.
        session
            .insert_client("c1".into(), "C".into(), &[CALLBACK.into()])
            .await
            .unwrap();
        let client = mcp.use_client("c1").await.unwrap().unwrap();
        assert_eq!(client.redirect_uris, vec![CALLBACK.to_string()]);
    }

    #[tokio::test]
    async fn sweep_removes_expired_rows_and_keeps_live_grants() {
        let (_dir, pool) = pool().await;
        let store = SqliteStore::new(pool.clone(), REALM_MCP);
        let count = |table: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {table}"))
                    .fetch_one(&pool)
                    .await
                    .unwrap()
            }
        };

        // Live rows.
        let live_pending = store.insert_pending(new_pending()).await.unwrap();
        store
            .insert_client("live".into(), "L".into(), &[])
            .await
            .unwrap();
        let live = store.create_grant(new_grant_for(alice())).await.unwrap();
        store
            .insert_refresh_token(&live, "live-rt".into())
            .await
            .unwrap();
        store
            .store_document("https://a/doc", json!({}), Utc::now() + Duration::hours(1))
            .await
            .unwrap();
        assert_eq!(store.sweep().await.unwrap(), 0);

        // Age everything: pending, devices, codes, documents and the
        // unused client expire; `live` (30 days) and the old grants do not
        // all go.
        let p = store.insert_pending(new_pending()).await.unwrap();
        let old = store
            .create_grant_with_code(
                &p,
                &alice(),
                Duration::days(1),
                "code-h".into(),
                Duration::minutes(1),
            )
            .await
            .unwrap();
        store
            .insert_refresh_token(&old, "old-rt".into())
            .await
            .unwrap();
        let revoked = store.create_grant(new_grant_for(alice())).await.unwrap();
        store.revoke_grant(revoked.id, "logout").await.unwrap();
        store
            .insert_device(new_device("d", "BCDF-GHJK"))
            .await
            .unwrap()
            .unwrap();
        store
            .insert_client("unused".into(), "U".into(), &[])
            .await
            .unwrap();
        store
            .insert_client("scottyctl".into(), "used".into(), &[])
            .await
            .unwrap();

        for (table, col) in [
            ("oauth_pending", "expires_at"),
            ("oauth_codes", "expires_at"),
            ("oauth_devices", "expires_at"),
            ("oauth_documents", "expires_at"),
            ("oauth_clients", "created_at"),
        ] {
            age(&pool, table, col, Duration::days(8)).await;
        }
        // `old` expired 31 days ago, `revoked` was revoked 31 days ago.
        sqlx::query("UPDATE oauth_grants SET expires_at = ? WHERE id = ?")
            .bind(ms(Utc::now() - Duration::days(31)))
            .bind(old.id.to_string())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE oauth_grants SET revoked_at = ? WHERE id = ?")
            .bind(ms(Utc::now() - Duration::days(31)))
            .bind(revoked.id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        // 2 pending + 1 code + 1 device + 1 document + 2 grants + 2 clients
        // ("live", "unused"; "scottyctl" has grants).
        assert_eq!(store.sweep().await.unwrap(), 9);
        assert!(store.find_grant(live.id).await.unwrap().is_some());
        assert!(store.find_refresh_token("live-rt").await.unwrap().is_some());
        assert!(store.find_grant(old.id).await.unwrap().is_none());
        assert!(
            store.find_refresh_token("old-rt").await.unwrap().is_none(),
            "cascaded"
        );
        assert!(store
            .claim_pending(live_pending.id, &alice())
            .await
            .unwrap()
            .is_none());
        assert_eq!(count("oauth_pending").await, 0);
        assert_eq!(count("oauth_devices").await, 0);
        assert_eq!(count("oauth_codes").await, 0);
        assert_eq!(count("oauth_documents").await, 0);
        assert_eq!(count("oauth_clients").await, 1);
        assert_eq!(count("oauth_grants").await, 1);
    }

    // --- The flows over the store -------------------------------------------

    struct Everyone;

    #[async_trait]
    impl Subjects<String> for Everyone {
        async fn recheck(&self, _grant: &Grant<String>) -> Result<Recheck, BackendError> {
            Ok(Recheck::Allow(Map::new()))
        }
    }

    struct NoFetch;

    #[async_trait]
    impl MetadataFetcher for NoFetch {
        async fn fetch(&self, _url: &Url, _limit: usize) -> Result<Fetched, FetchError> {
            Err(FetchError("no fetching in tests".into()))
        }
    }

    fn server(pool: SqlitePool) -> Server<String> {
        let config = Config {
            resource: RESOURCE.into(),
            token_kind: "mcp".into(),
            scopes: ScopeSpec::new(&["read", "offline_access"], &["read"], "offline_access"),
            signer: Signer::hmac(
                jsonwebtoken::Algorithm::HS512,
                b"0123456789abcdef0123456789abcdef",
            ),
            access_token_secs: 900,
            code_ttl: Duration::seconds(60),
            pending_ttl: Duration::seconds(600),
            grant_lifetime: Duration::days(30),
            documents: DocumentPolicy::default(),
        };
        Server::new(
            config,
            Arc::new(SqliteStore::new(pool, REALM_MCP)),
            Arc::new(Everyone),
            Arc::new(NoEvents),
            Arc::new(NoFetch),
        )
    }

    fn form(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn param(url: &str, name: &str) -> String {
        Url::parse(url)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
            .unwrap()
    }

    #[tokio::test]
    async fn authorization_code_and_refresh_flow() {
        let (_dir, pool) = pool().await;
        let server = server(pool);
        let doc = json!({"client_name": "Claude", "redirect_uris": [CALLBACK]});
        let client_id = server
            .register(doc.as_object().unwrap())
            .await
            .unwrap()
            .client_id;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()));
        let query = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("response_type", "code"),
                ("client_id", &client_id),
                ("redirect_uri", CALLBACK),
                ("code_challenge", &challenge),
                ("code_challenge_method", "S256"),
                ("resource", RESOURCE),
                ("scope", "read offline_access"),
                ("state", "xyz"),
            ])
            .finish();
        let pending = server.authorize(&query).await.unwrap();
        let approved = server.approve(pending.id, &alice(), None).await.unwrap();
        let code = param(&approved.redirect, "code");
        let exchange = form(&[
            ("code", &code),
            ("redirect_uri", CALLBACK),
            ("code_verifier", VERIFIER),
        ]);
        let tokens = server
            .exchange_code(&exchange, Some(&client_id), None)
            .await
            .unwrap();
        assert!(server.verify(&tokens.access_token).is_ok());
        assert!(matches!(
            server
                .exchange_code(&exchange, Some(&client_id), None)
                .await,
            Err(TokenError::InvalidGrant)
        ));
        // The code reuse revoked the grant; connect again for the refresh.
        let grant = server
            .store()
            .find_grant(approved.grant.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(grant.revoke_reason.as_deref(), Some("code_reuse"));

        let pending = server.authorize(&query).await.unwrap();
        let approved = server.approve(pending.id, &alice(), None).await.unwrap();
        let exchange = form(&[
            ("code", &param(&approved.redirect, "code")),
            ("redirect_uri", CALLBACK),
            ("code_verifier", VERIFIER),
        ]);
        let tokens = server
            .exchange_code(&exchange, Some(&client_id), None)
            .await
            .unwrap();
        let first = tokens.refresh_token.unwrap();
        let refresh = |token: &str| {
            let (server, form, client_id) = (
                server.clone(),
                form(&[("refresh_token", token)]),
                client_id.clone(),
            );
            async move { server.refresh(&form, Some(&client_id), None).await }
        };
        let next = refresh(&first).await.unwrap().refresh_token.unwrap();
        assert!(
            matches!(refresh(&first).await, Err(TokenError::InvalidGrant)),
            "reuse"
        );
        assert!(
            matches!(refresh(&next).await, Err(TokenError::InvalidGrant)),
            "grant revoked"
        );
        let grant = server
            .store()
            .find_grant(approved.grant.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(grant.revoke_reason.as_deref(), Some("refresh_reuse"));
    }

    #[tokio::test]
    async fn device_flow() {
        let (_dir, pool) = pool().await;
        let server = server(pool);
        let started = server
            .start_device("scottyctl", "scottyctl on host", "read offline_access")
            .await
            .unwrap();
        assert!(matches!(
            server
                .poll_device(&started.device_code, "scottyctl", None)
                .await,
            Err(PollError::Pending)
        ));
        assert!(server
            .approve_device(&started.device.user_code, &alice())
            .await
            .unwrap()
            .is_some());
        let tokens = server
            .poll_device(&started.device_code, "scottyctl", None)
            .await
            .unwrap();
        assert!(tokens.refresh_token.is_some());
        assert!(matches!(
            server
                .poll_device(&started.device_code, "scottyctl", None)
                .await,
            Err(PollError::Token(TokenError::InvalidGrant))
        ));
    }
}
