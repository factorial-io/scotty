# OAuth Authentication with OIDC

Scotty logs users in through an OIDC provider (GitLab, Auth0, Keycloak, ...)
and then issues its own tokens. The provider's tokens are used once, at login,
to read the user's identity. Scotty never accepts them as API credentials.

## Overview

Scotty supports three authentication modes configured via `auth_mode`:

- **`dev`**: Development mode with no authentication (uses fixed dev user)
- **`oauth`**: OIDC login, with Scotty-issued tokens for the web UI, `scottyctl` and MCP clients
- **`bearer`**: Traditional token-based authentication

In `oauth` mode Scotty is also an OAuth 2.1 authorization server. It issues
three kinds of credentials:

| Who | How they log in | Token | Lifetime |
|---|---|---|---|
| Web UI | OIDC login in the browser | session token (audience `scotty-api`) | 12 h, no refresh |
| `scottyctl` | device login, approved on Scotty's `/device` page | session token plus refresh token | 12 h access token, refreshed silently, 30-day limit |
| MCP clients (claude.ai, Claude Code) | authorization code with PKCE, approved on Scotty's consent page | MCP token (audience `{base_url}/mcp`) plus refresh token | 15 min access token, rotating refresh, 30-day limit |

Every token belongs to a *grant*. Scotty checks on every request that the
grant is still active, so logout and revocation take effect at once. Configured
bearer tokens keep working next to these (see
[Hybrid Authentication](#hybrid-authentication-with-bearer-tokens)).

The REST API accepts session tokens and bearer tokens. `/mcp` accepts MCP
tokens and bearer tokens. Neither accepts provider (GitLab) tokens, and
neither accepts the other's user tokens.

Grants, clients, codes and refresh tokens live in a SQLite database
(`api.database_path`). Tokens are signed with `api.jwt_secret`.

## Upgrading from provider tokens

Older releases handed the provider's access token to the browser and to
`scottyctl`, and accepted any token the provider's userinfo endpoint accepted.
That has changed:

- **The API no longer accepts provider (GitLab) tokens.** Scripts that sent a
  GitLab token to Scotty must use a [bearer token](#hybrid-authentication-with-bearer-tokens)
  instead.
- **Everyone logs in again once.** Existing browser sessions and `scottyctl`
  logins stop working after the upgrade. Log in again in the web UI and run
  `scottyctl auth:login`.
- **Set `api.jwt_secret`.** Scotty does not start in `oauth` mode without it,
  or with fewer than 64 bytes.
- **`/oauth/authorize` ignores `redirect_uri`.** The provider always gets
  `api.oauth.redirect_url`, and the browser always returns to the frontend's
  `/oauth/callback`.
- **Keep the database on a volume.** See [Persistent database](#persistent-database).
- **The provider application needs less.** It only needs the scopes
  `openid profile email`. It no longer needs the device authorization grant,
  and the `read_user` scope is no longer used.
- **Logout revokes the session.** The token stops working on the server, not
  only in the browser.

Older `scottyctl` versions can still log in. They ignore the refresh token and
must log in again after 12 h.

## How OAuth Mode Works

### Browser login

1. The frontend sends the user to Scotty's `/oauth/authorize`.
2. Scotty redirects to the OIDC provider with PKCE and a nonce.
3. The provider redirects back to `/api/oauth/callback` with a code.
4. Scotty exchanges the code and verifies the ID token (signature via JWKS,
   issuer, audience, nonce, expiry). It reads the email from the ID token and
   asks the userinfo endpoint only when the ID token has no email. Without an
   email, or with `email_verified: false`, the login fails. A missing
   `email_verified` counts as verified, so use a provider that only hands out
   addresses it has verified (GitLab does): roles are assigned by email.
5. Scotty drops the provider's tokens and creates a grant for client
   `scotty-web`.
6. The frontend picks up the session token once through `/oauth/exchange` and
   keeps it in `sessionStorage`.

The session token is valid for 12 hours. When it expires or is revoked, the
frontend sends the user to log in again (silently, if the provider session is
still alive). Logout calls `POST /api/v1/authenticated/logout`, which revokes
the grant.

### CLI device login

`scottyctl auth:login` uses the OAuth device authorization grant (RFC 8628),
run by Scotty itself. The OIDC provider is not involved.

1. `scottyctl` calls `POST /oauth/device` and prints a URL and a user code.
2. You open Scotty's `/device` page (logged in to the web UI), check the code
   and the client name, and approve or deny.
3. `scottyctl` polls `POST /oauth/device/token` and receives a session token
   and a refresh token.

When the access token expires, `scottyctl` refreshes it on the next 401 and
retries the request once. Each refresh rotates the refresh token. Reusing an
old refresh token revokes the whole session. After 30 days the session ends
and you log in again.

The CLI session has the scope `api offline_access` (it gets a refresh token);
the browser session has `api` only and no refresh token.

Turn the device login off with `api.oauth.device_flow_enabled: false`. Then
`/oauth/device` answers 404 and `scottyctl auth:login` points users to the web
interface.

### Role check

Scotty checks the authorization policy every time it issues or refreshes a token. A user who no
longer has any role loses the session at the next refresh, and the grant is
revoked.

### Route Protection

- **Public**: `/`, `/api/v1/health`, `/api/v1/info`, `/api/v1/login`, `/oauth/*`, `/.well-known/*`, static assets, SPA routes
- **Session or bearer token**: `/api/v1/authenticated/*`
- **MCP or bearer token**: `/mcp`

## Setup Instructions

### 1. OIDC Provider OAuth Application

Configure your OIDC provider (GitLab, Auth0, Keycloak, etc.):

#### GitLab Example:
1. Go to GitLab → Settings → Applications
2. Create new application:
   - **Name**: Scotty
   - **Redirect URI**: `http://localhost:21342/api/oauth/callback`
   - **Scopes**: `openid`, `profile`, `email`
3. Save the **Application ID** and **Secret**

The application does not need the device authorization grant.

#### Other OIDC Providers:
- **Auth0**: Create application in Auth0 dashboard
- **Keycloak**: Create client in Keycloak admin console
- **Google**: Use Google Cloud Console OAuth 2.0 setup

### 2. Scotty Configuration

Configure Scotty for OAuth mode in `config/local.yaml`:

```yaml
api:
  bind_address: "0.0.0.0:21342"
  auth_mode: "oauth"
  base_url: "http://localhost:21342"  # Public URL of Scotty
  database_path: "data/scotty.sqlite"  # on a persistent volume
  # jwt_secret: set via SCOTTY__API__JWT_SECRET
  oauth:
    oidc_issuer_url: "https://gitlab.com"  # or your OIDC provider URL
    client_id: "your_oidc_application_id"
    client_secret: "your_oidc_application_secret"
    redirect_url: "http://localhost:21342/api/oauth/callback"
```

**Provider-specific examples:**

```yaml
# GitLab
oauth:
  oidc_issuer_url: "https://gitlab.com"

# Auth0
oauth:
  oidc_issuer_url: "https://your-domain.auth0.com"

# Keycloak
oauth:
  oidc_issuer_url: "https://your-keycloak.com/auth/realms/your-realm"

# Google
oauth:
  oidc_issuer_url: "https://accounts.google.com"
```

### 3. Environment Variables

Alternatively, you can use environment variables:

```bash
# Set authentication mode
SCOTTY__API__AUTH_MODE=oauth

# Signing secret for Scotty's tokens (required, at least 64 bytes)
SCOTTY__API__JWT_SECRET=$(openssl rand -base64 48)

# OIDC OAuth Application credentials
SCOTTY__API__OAUTH__CLIENT_ID=your_oidc_application_id
SCOTTY__API__OAUTH__CLIENT_SECRET=your_oidc_application_secret

# OAuth configuration
SCOTTY__API__OAUTH__OIDC_ISSUER_URL=https://gitlab.com
SCOTTY__API__OAUTH__REDIRECT_URL=http://localhost:21342/api/oauth/callback

# Public base URL of Scotty
SCOTTY__API__BASE_URL=http://localhost:21342
```

The older `SCOTTY__API__OAUTH__FRONTEND_BASE_URL` still works as a deprecated
fallback when `SCOTTY__API__BASE_URL` is unset (see below).

### 4. Signing secret

`api.jwt_secret` signs every token Scotty issues (HS512). It is required in
`oauth` mode and must be at least 64 bytes (RFC 7518 §3.2 for HS512); Scotty refuses to start otherwise.
Set it through `SCOTTY__API__JWT_SECRET` and keep it with your other secrets.

If the secret leaks, anyone can forge tokens. Rotate it: set a new value and
restart Scotty. All existing tokens stop working, so every browser, CLI and
MCP session must log in again. That is the emergency step.

### Persistent database

`api.database_path` (default `data/scotty.sqlite`, relative to Scotty's
working directory) holds the grants, registered clients, codes and refresh
tokens. Scotty creates the file and its directory at startup and runs SQLite in
WAL mode.

Put this path on a persistent, writable volume. If the file is lost, for
example because the container is recreated without a volume, every CLI and MCP
session is gone and all users must log in and connect again. App data is not
affected. In the Docker image the working directory is `/app`:

```yaml
services:
  scotty:
    volumes:
      - ./scotty-data:/app/data
```

WAL mode writes `-wal` and `-shm` files next to the database, so mount the
directory, not only the file.

Run a single Scotty instance. Logins in progress (the browser flow's PKCE state
and the one-time `/oauth/exchange` session) live in memory: a restart drops
them, and a second replica would not see them. Expired entries are swept
periodically.

### Understanding OAuth URLs

Scotty uses two different URL configurations for OAuth:

#### `redirect_url` - Backend OAuth Callback
- **Purpose**: The OAuth callback endpoint where the OIDC provider redirects after user authentication
- **Used by**: OIDC provider (GitLab, Auth0, etc.)
- **Must match**: The redirect URI configured in your OIDC provider's OAuth application settings
- **Example**: `http://localhost:21342/api/oauth/callback`
- **Format**: Full URL to Scotty's backend `/api/oauth/callback` endpoint

#### `api.base_url` - Public Base URL of Scotty
- **Purpose**: The public base URL of Scotty, where users are redirected after OAuth completes
- **Used by**: Scotty backend to redirect users back to the web interface with a session ID, the device and consent pages, the MCP resource URL `{base_url}/mcp` (and by the [default backend / landing page](default-backend.md) feature)
- **Must match**: The actual URL where your users access Scotty's web interface
- **Example**: `http://localhost:21342` (development) or `https://scotty.example.com` (production)
- **Format**: Base URL only (no path) - Scotty appends `/oauth/callback?session_id=xyz`

**Production Example:**
```yaml
api:
  base_url: "https://scotty.example.com"
  oauth:
    redirect_url: "https://scotty.example.com/api/oauth/callback"
```

**Important**: Both URLs must match your production domain. Using `localhost` in production will break the OAuth flow — Scotty logs a warning at startup when `api.base_url` is not configured and the `localhost` default is in effect.

> **Deprecated:** older releases used `api.oauth.frontend_base_url` for the post-login redirect. It still works as a fallback when `api.base_url` is unset, but logs a deprecation warning at startup; if both are set, `api.base_url` wins. Having two separately-configured URLs caused hard-to-diagnose bugs (e.g. the stopped-app landing page losing its "return to app" state when the two origins differed), which is why they were consolidated.

## OAuth Endpoints

### Browser login

- `GET /oauth/authorize`: starts the OIDC login. Optional `redirect_uri` query parameter for the page to return to.
- `GET /api/oauth/callback`: the provider's callback. Verifies the ID token and creates the session.
- `POST /oauth/exchange`: returns the session token once for a `session_id`. The response contains no provider token.
- `POST /api/v1/authenticated/logout`: revokes the session.

### CLI device login

- `POST /oauth/device`: starts a device login.
- `POST /oauth/device/token`: polls (`grant_type=urn:ietf:params:oauth:grant-type:device_code`) and refreshes (`grant_type=refresh_token`).
- `GET /api/v1/authenticated/oauth/device/{user_code}`, `POST .../oauth/device/approve`, `POST .../oauth/device/deny`: used by the `/device` page.

### MCP authorization server

- `GET /.well-known/oauth-authorization-server`: authorization server metadata (RFC 8414).
- `GET /.well-known/oauth-protected-resource/mcp`: protected resource metadata (RFC 9728).
- `GET /oauth/mcp/authorize`: authorization code request (PKCE required).
- `POST /oauth/mcp/token`: code exchange and refresh.
- `POST /oauth/mcp/register`: dynamic client registration (RFC 7591), rate limited.
- `/mcp`: the MCP endpoint (see [MCP Server](mcp.md)).
- `GET /api/v1/authenticated/oauth/consent/{id}`, `POST .../approve`, `POST .../deny`: used by the consent page.
- `GET /api/v1/authenticated/connections`, `DELETE /api/v1/authenticated/connections/{id}`: used by the connections page.

## MCP connections

MCP clients such as claude.ai and Claude Code connect to `{base_url}/mcp`.
The tools it offers are described in [MCP Server](mcp.md).

### How a client connects

1. The client calls `/mcp`, gets a 401 that points to the protected resource
   metadata, and discovers Scotty's authorization server.
2. The client identifies itself with a Client ID Metadata Document (an `https`
   URL as `client_id`) or registers at `/oauth/mcp/register`.
3. Your browser opens Scotty's consent page (`/oauth/consent/{id}`). If you are
   not logged in, you log in first.
4. The page shows the host the client redirects to, the name it gives itself,
   and the scopes it asks for. Untick scopes you do not want to grant, then
   approve or deny.
5. The client receives a 15-minute access token and a rotating refresh token.
   The connection lasts at most 30 days.

### Scopes

| Scope | Allows |
|---|---|
| `read` | view apps, blueprints and tasks; read logs |
| `create` | create apps |
| `destroy` | destroy apps |
| `offline_access` | a refresh token, so the connection survives the 15-minute token |

A tool runs only if the token has the scope **and** the policy gives you the
permission for that app. A connection never gets more than you have.

### Connections page

The **Connections** page (`/connections`) lists your `scottyctl` sessions and
MCP connections with client, redirect host, scopes, creation and last use.
Revoking one stops its refresh and its access at once. Browser sessions are not
listed. Users with `admin_write` can switch to all users' connections and
revoke any of them.

`scottyctl auth:logout` revokes the CLI session on the server and then deletes
the local credentials. If the server cannot be reached, it still deletes them
and tells you to revoke the session on the connections page.

### Claude Code

Add Scotty as a remote HTTP MCP server
([Claude Code MCP docs](https://code.claude.com/docs/en/mcp)):

```bash
claude mcp add --transport http scotty https://scotty.example.com/mcp
```

Then run `/mcp` inside Claude Code and choose to authenticate. Your browser
opens Scotty's consent page.

### claude.ai

Add Scotty as a custom connector
([Anthropic help: custom connectors](https://support.claude.com/en/articles/11175166-getting-started-with-custom-connectors-using-remote-mcp)):

- **Pro and Max**: Customize → Connectors → **+ Add** → Add custom connector.
  Enter a name and `https://scotty.example.com/mcp`.
- **Team and Enterprise**: an owner adds it under Organization settings →
  Connectors → Add → Custom → Web. Members then connect it under Customize →
  Connectors.

Leave the OAuth client fields empty; Scotty identifies claude.ai through its
metadata document or dynamic registration.

### Network access for claude.ai

claude.ai calls Scotty from Anthropic's servers, not from your browser. A
firewalled Scotty must let Anthropic's outbound addresses reach:

- `/mcp`
- `/.well-known/*`
- `/oauth/mcp/register`
- `/oauth/mcp/token`

The consent page and `/oauth/mcp/authorize` are opened in your browser and
need no extra rule.

Anthropic publishes its outbound IP ranges at
[platform.claude.com/docs/en/api/ip-addresses](https://platform.claude.com/docs/en/api/ip-addresses).
As of November 2026 the outbound IPv4 ranges are `160.79.104.0/21` and
`153.61.208.0/20`. Check that page before you write firewall rules; the ranges
can change.

Scotty itself fetches Client ID Metadata Documents from the `https` URL a
client gives as `client_id`. It fetches only from public addresses and follows
no redirects. A Scotty without outbound HTTPS access needs dynamic registration
instead.

### Registration rate limit

`/oauth/mcp/register` is always rate limited per client address in `oauth`
mode, even when `api.rate_limiting.enabled` is `false`. The defaults are 10
requests per minute with a burst of 5. If the `api.rate_limiting.oauth` tier
has `requests_per_minute` above 0, its numbers are used instead.

The limit keys on the first address in `X-Forwarded-For`, then `X-Real-IP`,
then the connection's peer address. **Run Scotty behind a proxy that sets
`X-Forwarded-For`** and replaces any value the client sent (Traefik does this
by default for untrusted sources). If clients can reach Scotty directly, they
can set the header themselves and pick their own rate-limit key.

## Development vs Production

### Development Setup

For local development, start Scotty with OAuth configuration:

```bash
# Set OAuth configuration
export SCOTTY__API__AUTH_MODE=oauth
export SCOTTY__API__JWT_SECRET=$(openssl rand -base64 48)
export SCOTTY__API__OAUTH__CLIENT_ID=your_client_id
export SCOTTY__API__OAUTH__CLIENT_SECRET=your_client_secret

# Run Scotty
cargo run --bin scotty
```

### Development Mode Alternative

For faster iteration during development, you can use `auth_mode: "dev"`:

```yaml
api:
  auth_mode: "dev"
  dev_user_email: "developer@localhost"
  dev_user_name: "Local Developer"
```

This bypasses OAuth and uses a fixed development user.

### Production Considerations

1. **Use HTTPS**: Configure TLS for your domain and update redirect URLs
2. **Proper domains**: Replace `localhost` with your actual domain in all configurations
3. **Secure secrets**: Set `jwt_secret` and `client_secret` through environment variables or a secret manager
4. **Persistent volume**: Keep `database_path` on a volume
5. **Proxy**: Put Scotty behind a proxy that sets `X-Forwarded-For`
6. **Firewall**: Allow claude.ai's outbound range if you use the claude.ai connector

## Hybrid Authentication with Bearer Tokens

**Use Case:** OAuth for human users + bearer tokens for service accounts (CI/CD, monitoring, automation).

### Overview

When `auth_mode` is set to `oauth`, Scotty supports an optional **bearer token fallback** for service accounts. This hybrid approach provides:

- **OAuth authentication** for human users (web UI, CLI via device login)
- **Bearer token authentication** for service accounts (CI/CD pipelines, monitoring tools, automation scripts)
- **Single authentication mode** - no need to switch between modes

### How Hybrid Authentication Works

1. **Extract token** from `Authorization: Bearer <token>` header
2. **Check bearer tokens first** (HashMap lookup) → if found, authenticate as service account
3. **Verify a Scotty session token** (signature, audience, expiry) and check that its grant is active
4. **Return authentication result** or 401 Unauthorized

No step calls the OIDC provider.

### Configuration

Enable hybrid authentication by configuring both OAuth and bearer tokens:

```yaml
api:
  auth_mode: oauth  # Enable OAuth mode
  base_url: "http://localhost:21342"

  # OAuth configuration for human users
  oauth:
    oidc_issuer_url: "https://gitlab.com"
    client_id: "your_oidc_application_id"
    client_secret: "your_oidc_application_secret"
    redirect_url: "http://localhost:21342/api/oauth/callback"

  # Bearer tokens for service accounts (checked first)
  bearer_tokens:
    ci-bot: "OVERRIDE_VIA_ENV_VAR"        # CI/CD pipeline
    monitoring: "OVERRIDE_VIA_ENV_VAR"    # Monitoring service
    deployment: "OVERRIDE_VIA_ENV_VAR"    # Deployment automation
```

**Environment Variable Configuration:**

```bash
# Set OAuth credentials and the signing secret
export SCOTTY__API__OAUTH__CLIENT_ID=your_client_id
export SCOTTY__API__OAUTH__CLIENT_SECRET=your_client_secret
export SCOTTY__API__JWT_SECRET=$(openssl rand -base64 48)

# Set bearer tokens for service accounts (IMPORTANT: Use secure random tokens!)
# Note: Hyphens in config keys (ci-bot) become underscores in env vars (CI_BOT)
export SCOTTY__API__BEARER_TOKENS__CI_BOT=$(openssl rand -base64 32)
export SCOTTY__API__BEARER_TOKENS__MONITORING=$(openssl rand -base64 32)
export SCOTTY__API__BEARER_TOKENS__DEPLOYMENT=$(openssl rand -base64 32)

# Start Scotty
cargo run --bin scotty
```

### RBAC Configuration for Service Accounts

Configure authorization for bearer token identifiers in `config/casbin/policy.yaml`:

```yaml
# Scopes and roles definitions
scopes:
  production:
    description: Production environment
    created_at: "2023-12-01T00:00:00Z"
  staging:
    description: Staging environment
    created_at: "2023-12-01T00:00:00Z"

roles:
  admin:
    description: Full access
    permissions: ["*"]
  deployer:
    description: Deploys apps
    permissions: [view, manage, create]
  monitor:
    description: Read-only monitoring
    permissions: [view, logs]

# Authorization assignments
assignments:
  # OAuth users (human users)
  "alice@example.com":
    - role: admin
      scopes: ["*"]

  # Bearer token service accounts
  # Format: identifier:<token_name> (matches bearer_tokens key)
  "identifier:ci-bot":
    - role: deployer
      scopes: [staging, production]

  "identifier:monitoring":
    - role: monitor
      scopes: ["*"]

  "identifier:deployment":
    - role: deployer
      scopes: [production]
```

**Key Concept:** The `identifier:<name>` in policy.yaml maps to the key in `bearer_tokens` configuration, NOT the actual token value.

**Example mapping:**
- Configuration: `bearer_tokens.ci-bot: "secret-token-abc123"`
- Policy: `identifier:ci-bot: [role: deployer, ...]`
- API call: `Authorization: Bearer secret-token-abc123`
- Scotty maps: token → identifier "ci-bot" → role "deployer"

### Usage Examples

**Human User (OAuth):**
```bash
# Web UI: Click "Login with OAuth" button
# CLI: Use the device login
scottyctl auth:login --server https://scotty.example.com

# Use authenticated commands
scottyctl app:list
```

**Service Account (Bearer Token):**
```bash
# CI/CD Pipeline
export SCOTTY_ACCESS_TOKEN="${CI_BOT_TOKEN}"  # From CI/CD secrets
scottyctl --server https://scotty.example.com app:create my-app

# Monitoring Script
curl -H "Authorization: Bearer ${MONITORING_TOKEN}" \
  https://scotty.example.com/api/v1/authenticated/apps

# Deployment Automation
export SCOTTY_SERVER=https://scotty.example.com
export SCOTTY_ACCESS_TOKEN="${DEPLOYMENT_TOKEN}"
scottyctl app:restart production-app
```

### Security Best Practices

1. **Generate Strong Tokens for Service Accounts**
   ```bash
   # Use cryptographically secure random tokens
   openssl rand -base64 32
   ```

2. **Never Commit Tokens to Git**
   - Use environment variables exclusively
   - Store in CI/CD secret management
   - Rotate regularly

3. **Use Descriptive Identifiers**
   ```yaml
   # Good: Semantic, descriptive names
   bearer_tokens:
     ci-bot: "..."
     monitoring-service: "..."
     deployment-automation: "..."

   # Bad: Confusing, token-like names
   bearer_tokens:
     test-bearer-token-123: "..."  # Looks like a token value!
   ```

4. **Apply Least Privilege via RBAC**
   - CI/CD bot: `deployer` role (no destroy permission)
   - Monitoring: `monitor` role (view and logs only)
   - Deployment: Limited to specific scopes

5. **Monitor and Audit**
   - Grant creation and revocation are logged at info level (`grant created`, `grant revoked`, with realm, client and user)
   - Monitor for suspicious activity
   - Rotate tokens on compromise

### Migration from Pure OAuth

**Scenario:** You have an existing OAuth-only deployment and want to add service accounts.

**Steps:**

1. **Add bearer_tokens configuration** (without changing auth_mode)
   ```yaml
   api:
     auth_mode: oauth  # Keep OAuth mode
     oauth:
       # ... existing OAuth config ...
     bearer_tokens:  # ADD service account tokens
       ci-bot: "OVERRIDE_VIA_ENV_VAR"
   ```

2. **Configure RBAC for service accounts** in `policy.yaml`
   ```yaml
   assignments:
     "identifier:ci-bot":
       - role: deployer
         scopes: [staging]
   ```

3. **Set environment variables** for bearer tokens
   ```bash
   export SCOTTY__API__BEARER_TOKENS__CI_BOT=$(openssl rand -base64 32)
   ```

4. **Restart Scotty** - existing OAuth users are not affected.

5. **Update CI/CD pipelines** to use bearer tokens
   ```yaml
   # .gitlab-ci.yml example
   deploy:
     script:
       - export SCOTTY_ACCESS_TOKEN="${CI_BOT_TOKEN}"
       - scottyctl app:create ...
   ```

6. **Test both authentication paths**
   ```bash
   # Test OAuth (human)
   scottyctl auth:login
   scottyctl app:list

   # Test bearer token (service account)
   export SCOTTY_ACCESS_TOKEN="${CI_BOT_TOKEN}"
   scottyctl app:list
   ```

### Troubleshooting

**Issue: Service account bearer token not working**
```
Error: 401 Unauthorized
```

**Diagnosis:**
1. Check token is correctly set:
   ```bash
   echo $SCOTTY_ACCESS_TOKEN  # Should show token value
   ```

2. Check identifier exists in policy.yaml:
   ```yaml
   assignments:
     "identifier:ci-bot":  # Must match bearer_tokens key
       - role: deployer
         scopes: [staging]
   ```

3. Check bearer_tokens configuration:
   ```bash
   # Verify token mapping
   curl http://localhost:21342/api/v1/info  # Should not show token values
   ```

4. Check logs with debug mode:
   ```bash
   RUST_LOG=debug cargo run --bin scotty
   # Look for: "Found identifier '...' for bearer token" or "Session token rejected"
   ```

**Issue: A GitLab token gets 401**

This is expected. Scotty no longer accepts provider tokens. Use the web UI,
`scottyctl auth:login` or a bearer token.

**Issue: OAuth users can't log in**

- Verify OAuth configuration is still correct (client_id, client_secret, etc.)
- Check OIDC provider is accessible
- Verify redirect URLs match
- Check that the provider puts an email in the ID token or returns it from userinfo; without an email the login fails

### When to Use Hybrid Authentication

**Use Hybrid Mode When:**
- ✅ You have both human users (web UI) and service accounts (CI/CD)
- ✅ You want centralized authentication with OIDC
- ✅ You want fine-grained RBAC for different actors

**Use Pure OAuth When:**
- ✅ Only human users access Scotty
- ✅ All access is via web UI, CLI device login or MCP clients
- ✅ No automation or service accounts needed

**Use Pure Bearer Mode When:**
- ✅ Only service accounts/automation access Scotty
- ✅ No human interactive access needed
- ✅ Simpler deployment without OAuth infrastructure

See [Configuration Documentation](configuration.html) for complete configuration reference and [config/README.md](../../config/README.md) for detailed security best practices.

## Frontend Integration

The Scotty frontend detects OAuth mode and provides:

- **Login page** with a "Continue with OAuth" button
- **OAuth callback page** that picks up the session token
- **`/device`** to approve or deny a `scottyctl` login
- **`/oauth/consent/{id}`** to approve or deny an MCP client
- **`/connections`** to list and revoke CLI and MCP connections

The session token is kept in `sessionStorage` (cleared when the tab closes).
On a 401 the frontend sends you to log in and back to the page you were on.

## CLI Integration (scottyctl)

```bash
scottyctl --server https://scotty.example.com auth:login
```

See [CLI authentication](cli.html#authentication) for details. Service
accounts use a bearer token in `SCOTTY_ACCESS_TOKEN`.

## Security Features

### PKCE and nonce
- The browser login uses PKCE (SHA256) and an OIDC nonce. Verifiers and nonces are stored in memory using the `MaskedSecret` type.
- MCP clients must use PKCE. Redirect URIs must match exactly (a loopback port may differ). Authorization codes are single use.

### CSRF Protection
- **State parameter validation** - combines session ID and CSRF token (`session_id:csrf_token` format)
- **Automatic cleanup** - expired login sessions are removed every 5 minutes; a periodic sweep removes expired codes, pending authorizations and clients registered more than 7 days ago that never got a grant
- **Time-limited sessions** - web flow sessions expire after 10 minutes, the exchange session after 5 minutes

### Token Security
- **Scotty-issued tokens only** - provider tokens are used once at login and then dropped
- **Grant check on every request** - logout and revocation take effect at once
- **Audience and kind** - session tokens work only on the REST API, MCP tokens only on `/mcp`
- **Refresh rotation** - reusing a refresh token revokes the grant
- **Absolute lifetime** - CLI and MCP grants end after 30 days
- **Secure storage** - the web session token is stored in `sessionStorage` (cleared when the tab closes)

## Troubleshooting

### Common Issues

**Scotty does not start**
```
api.jwt_secret is required in oauth mode (set SCOTTY__API__JWT_SECRET)
```
- Set `SCOTTY__API__JWT_SECRET` to at least 64 bytes, e.g. `openssl rand -base64 48`.

**Redirect URI Mismatch**
```
Error: redirect_uri mismatch in OIDC provider
```
- Ensure OIDC provider OAuth app redirect URI exactly matches Scotty configuration
- Check for trailing slashes, HTTP vs HTTPS, and port numbers
- Verify the redirect URI is `http://localhost:21342/api/oauth/callback`

**Invalid Client Credentials**
```
Error: Invalid client credentials
```
- Verify `client_id` and `client_secret` match OIDC provider OAuth application
- Ensure credentials are correctly set in configuration or environment variables

**Session Expired**
```
Error: OAuth session not found or expired
```
- OAuth sessions have a limited lifetime
- Restart the authentication flow from the beginning

**Everyone is logged out after a restart**
- The database was not on a persistent volume, or `jwt_secret` changed. See [Persistent database](#persistent-database).

**claude.ai cannot connect**
- Check that Anthropic's outbound range can reach `/mcp`, `/.well-known/*`, `/oauth/mcp/register` and `/oauth/mcp/token`.
- Check that `api.base_url` is the public HTTPS URL.

### Debug Commands

```bash
# Check Scotty configuration
curl http://localhost:21342/api/v1/info

# MCP discovery
curl http://localhost:21342/.well-known/oauth-authorization-server
curl http://localhost:21342/.well-known/oauth-protected-resource/mcp

# Verify authentication (with a session or bearer token)
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:21342/api/v1/authenticated/apps
```

### Debug Logging

Enable debug logging to troubleshoot OAuth issues:

```bash
RUST_LOG=debug cargo run --bin scotty
```

## URLs and Access

- **Application**: http://localhost:21342
- **OAuth Authorization**: http://localhost:21342/oauth/authorize
- **OAuth Callback**: http://localhost:21342/api/oauth/callback
- **OAuth Session Exchange**: http://localhost:21342/oauth/exchange
- **Device approval**: http://localhost:21342/device
- **Connections**: http://localhost:21342/connections
- **MCP endpoint**: http://localhost:21342/mcp
- **API Documentation**: http://localhost:21342/rapidoc
- **Health Check**: http://localhost:21342/api/v1/health (public)
