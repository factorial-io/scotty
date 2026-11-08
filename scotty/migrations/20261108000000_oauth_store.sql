-- grantry OAuthStore (scotty/src/oauth/store.rs).
-- Timestamps are INTEGER unix milliseconds, ids are TEXT UUIDs.
-- `realm` separates the session and mcp servers; clients and cached
-- documents are shared.

CREATE TABLE oauth_clients (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    redirect_uris TEXT NOT NULL, -- JSON array
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER
);

CREATE TABLE oauth_documents (
    url        TEXT PRIMARY KEY,
    document   TEXT NOT NULL, -- JSON
    expires_at INTEGER NOT NULL
);

CREATE TABLE oauth_pending (
    id             TEXT PRIMARY KEY,
    realm          TEXT NOT NULL,
    client_id      TEXT NOT NULL,
    client_name    TEXT NOT NULL,
    loopback_only  INTEGER NOT NULL,
    redirect_uri   TEXT NOT NULL,
    scope          TEXT NOT NULL,
    state          TEXT,
    code_challenge TEXT NOT NULL,
    resource       TEXT NOT NULL,
    subject        TEXT,
    created_at     INTEGER NOT NULL,
    expires_at     INTEGER NOT NULL
);

CREATE TABLE oauth_grants (
    id                TEXT PRIMARY KEY,
    realm             TEXT NOT NULL,
    subject           TEXT NOT NULL,
    client_id         TEXT NOT NULL,
    client_name       TEXT NOT NULL,
    redirect_uri      TEXT NOT NULL,
    scope             TEXT NOT NULL,
    created_at        INTEGER NOT NULL,
    expires_at        INTEGER NOT NULL,
    last_refreshed_at INTEGER,
    revoked_at        INTEGER,
    revoke_reason     TEXT
);
CREATE INDEX oauth_grants_realm_subject ON oauth_grants (realm, subject);
CREATE INDEX oauth_grants_client ON oauth_grants (client_id);

CREATE TABLE oauth_codes (
    id             TEXT PRIMARY KEY,
    realm          TEXT NOT NULL,
    code_hash      TEXT NOT NULL UNIQUE,
    grant_id       TEXT NOT NULL REFERENCES oauth_grants (id) ON DELETE CASCADE,
    client_id      TEXT NOT NULL,
    redirect_uri   TEXT NOT NULL,
    resource       TEXT NOT NULL,
    code_challenge TEXT NOT NULL,
    expires_at     INTEGER NOT NULL,
    used_at        INTEGER
);
CREATE INDEX oauth_codes_grant ON oauth_codes (grant_id);

CREATE TABLE oauth_refresh_tokens (
    id         TEXT PRIMARY KEY,
    realm      TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    grant_id   TEXT NOT NULL REFERENCES oauth_grants (id) ON DELETE CASCADE,
    issued_at  INTEGER NOT NULL,
    rotated_at INTEGER
);
CREATE INDEX oauth_refresh_tokens_grant ON oauth_refresh_tokens (grant_id);

CREATE TABLE oauth_devices (
    id               TEXT PRIMARY KEY,
    realm            TEXT NOT NULL,
    device_code_hash TEXT NOT NULL UNIQUE,
    client_id        TEXT NOT NULL,
    client_name      TEXT NOT NULL,
    scope            TEXT NOT NULL,
    user_code        TEXT NOT NULL,
    status           TEXT NOT NULL,
    interval_seconds INTEGER NOT NULL,
    last_polled_at   INTEGER,
    subject          TEXT,
    created_at       INTEGER NOT NULL,
    expires_at       INTEGER NOT NULL
);
CREATE INDEX oauth_devices_user_code ON oauth_devices (realm, user_code);
