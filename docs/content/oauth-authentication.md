# OAuth Authentication with OIDC

Scotty provides built-in OAuth authentication with OIDC (OpenID Connect) integration. This setup offers secure authentication that protects your Scotty API endpoints while providing a seamless user experience through the web interface.

## Overview

Scotty supports three authentication modes configured via `auth_mode`:

- **`dev`**: Development mode with no authentication (uses fixed dev user)
- **`oauth`**: Native OAuth authentication with OIDC provider integration  
- **`bearer`**: Traditional token-based authentication

In OAuth mode, Scotty handles the complete OAuth 2.0 Authorization Code flow with PKCE (Proof Key for Code Exchange) for enhanced security.

## How OAuth Mode Works

### Architecture

```
User → Frontend SPA → Scotty OAuth Endpoints → OIDC Provider
                              ↓
                        Session Management
                              ↓  
                         User Authentication
```

### Authentication Flow

1. **User initiates login** via the Scotty frontend
2. **Frontend redirects** to Scotty's `/oauth/authorize` endpoint
3. **Scotty generates** authorization URL with PKCE challenge and redirects to OIDC provider
4. **User authenticates** with OIDC provider
5. **OIDC provider redirects** back to Scotty's `/api/oauth/callback` endpoint with authorization code
6. **Scotty exchanges** authorization code for access token using PKCE verifier
7. **User information** is extracted via OIDC `/oauth/userinfo` endpoint and session is created
8. **Frontend exchanges** session for tokens and stores OAuth tokens and user info in localStorage

### Route Protection

- **Public**: `/`, `/api/v1/health`, `/api/v1/info`, `/api/v1/login`, `/oauth/*`, static assets, SPA routes
- **Protected**: `/api/v1/authenticated/*` - all API operations that modify state

## Setup Instructions

### 1. OIDC Provider OAuth Application

Configure your OIDC provider (GitLab, Auth0, Keycloak, etc.):

#### GitLab Example:
1. Go to GitLab → Settings → Applications  
2. Create new application:
   - **Name**: Scotty  
   - **Redirect URI**: `http://localhost:21342/api/oauth/callback`
   - **Scopes**: `openid`, `profile`, `email`, `read_user`
3. Save the **Application ID** and **Secret**

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
  oauth:
    oidc_issuer_url: "https://gitlab.com"  # or your OIDC provider URL
    client_id: "your_oidc_application_id"
    client_secret: "your_oidc_application_secret"
    redirect_url: "http://localhost:21342/api/oauth/callback"
    frontend_base_url: "http://localhost:21342"  # Base URL for frontend redirects (default: http://localhost:21342)
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

# OIDC OAuth Application credentials  
SCOTTY__API__OAUTH__CLIENT_ID=your_oidc_application_id
SCOTTY__API__OAUTH__CLIENT_SECRET=your_oidc_application_secret

# OAuth configuration
SCOTTY__API__OAUTH__OIDC_ISSUER_URL=https://gitlab.com
SCOTTY__API__OAUTH__REDIRECT_URL=http://localhost:21342/api/oauth/callback
SCOTTY__API__OAUTH__FRONTEND_BASE_URL=http://localhost:21342
```

### Understanding OAuth URLs

Scotty uses two different URL configurations for OAuth:

#### `redirect_url` - Backend OAuth Callback
- **Purpose**: The OAuth callback endpoint where the OIDC provider redirects after user authentication
- **Used by**: OIDC provider (GitLab, Auth0, etc.)
- **Must match**: The redirect URI configured in your OIDC provider's OAuth application settings
- **Example**: `http://localhost:21342/api/oauth/callback`
- **Format**: Full URL to Scotty's backend `/api/oauth/callback` endpoint

#### `frontend_base_url` - Frontend Application Base URL
- **Purpose**: The base URL of your frontend application where users are redirected after OAuth completes
- **Used by**: Scotty backend to redirect users back to the frontend with session ID
- **Must match**: The actual URL where your users access Scotty's web interface
- **Example**: `http://localhost:21342` (development) or `https://scotty.example.com` (production)
- **Format**: Base URL only (no path) - Scotty appends `/oauth/callback?session_id=xyz`

**Production Example:**
```yaml
oauth:
  redirect_url: "https://scotty.example.com/api/oauth/callback"
  frontend_base_url: "https://scotty.example.com"
```

**Important**: Both URLs must match your production domain. Using `localhost` in production will break the OAuth flow.

## OAuth Endpoints

Scotty provides the following OAuth endpoints:

### `GET /oauth/authorize`

Initiates the OAuth authorization flow. Redirects to OIDC provider with proper PKCE parameters.

**Query Parameters:**
- `redirect_uri` (optional): Where to redirect after successful authentication

### `GET /api/oauth/callback`

Handles the OAuth callback from OIDC provider. Exchanges authorization code for access token and creates temporary session.

**Query Parameters:**
- `code`: Authorization code from OIDC provider
- `state`: CSRF protection token with embedded session ID

### `POST /oauth/exchange`

Exchanges temporary session for OAuth tokens (used by frontend).

**Request Body:**
- `session_id`: Temporary session identifier

## User Information

After successful OAuth authentication, Scotty provides OIDC-standard user information:

- **Subject (sub)**: OIDC user ID (typically a string)
- **Username**: Preferred username (optional)  
- **Name**: User's display name (optional)
- **Email**: User's email address (optional)
- **Access Token**: OAuth access token for API calls

This information is available to both the frontend (stored in sessionStorage) and backend (through authentication middleware).

## Development vs Production

### Development Setup

For local development, start Scotty with OAuth configuration:

```bash
# Set OAuth configuration
export SCOTTY__API__AUTH_MODE=oauth
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
3. **Secure secrets**: Use environment variables or secret management systems
4. **CORS configuration**: Ensure proper CORS settings for your domain
5. **Session security**: Configure appropriate session timeouts

## Hybrid Authentication: OAuth + Bearer Tokens

**Use Case:** OAuth for human users + bearer tokens for service accounts (CI/CD, monitoring, automation).

### Overview

When `auth_mode` is set to `oauth`, Scotty supports an optional **bearer token fallback** for service accounts. This hybrid approach provides:

- **OAuth authentication** for human users (web UI, CLI via device flow)
- **Bearer token authentication** for service accounts (CI/CD pipelines, monitoring tools, automation scripts)
- **Single authentication mode** - no need to switch between modes

### How Hybrid Authentication Works

**Authentication Flow (Optimized for Performance):**

1. **Extract token** from `Authorization: Bearer <token>` header
2. **Check bearer tokens FIRST** (fast HashMap lookup) → If found, authenticate as service account
3. **Try OAuth validation** (network call to OIDC provider) → If bearer check fails
4. **Return authentication result** or 401 Unauthorized

This order ensures **service accounts experience zero OAuth latency** while still supporting OAuth for human users.

### Configuration

Enable hybrid authentication by configuring both OAuth and bearer tokens:

```yaml
api:
  auth_mode: oauth  # Enable OAuth mode

  # OAuth configuration for human users
  oauth:
    oidc_issuer_url: "https://gitlab.com"
    client_id: "your_oidc_application_id"
    client_secret: "your_oidc_application_secret"
    redirect_url: "http://localhost:21342/api/oauth/callback"
    frontend_base_url: "http://localhost:21342"

  # Bearer tokens for service accounts (checked first for performance)
  bearer_tokens:
    ci-bot: "OVERRIDE_VIA_ENV_VAR"        # CI/CD pipeline
    monitoring: "OVERRIDE_VIA_ENV_VAR"    # Monitoring service
    deployment: "OVERRIDE_VIA_ENV_VAR"    # Deployment automation
```

**Environment Variable Configuration:**

```bash
# Set OAuth credentials
export SCOTTY__API__OAUTH__CLIENT_ID=your_client_id
export SCOTTY__API__OAUTH__CLIENT_SECRET=your_client_secret

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
  staging:
    description: Staging environment

roles:
  deployer:
    permissions: [view, manage, create]
  monitor:
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
# CLI: Use device flow
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
   - Track which authentication method is used (logs show "Bearer token authentication successful" vs "OAuth authentication successful")
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

4. **Restart Scotty** - No breaking changes to existing OAuth users!

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

**Zero Downtime:** Existing OAuth users continue working without any changes. Service accounts can be added incrementally.

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
   # Look for: "Bearer token authentication successful" or "OAuth authentication successful"
   ```

5. Verify expected behavior matches tests:
   ```bash
   # See test_oauth_bearer_token_fallback_with_valid_token in:
   # scotty/src/api/bearer_auth_tests.rs:233-257
   cargo test test_oauth_bearer_token_fallback_with_valid_token -- --nocapture
   ```

**Issue: OAuth users can't authenticate**
```
Error: OAuth validation failed
```

**Diagnosis:**
- Verify OAuth configuration is still correct (client_id, client_secret, etc.)
- Check OIDC provider is accessible
- Verify redirect URLs match

**Issue: Unclear which authentication method was used**

**Solution:** Check Scotty logs - authentication method is logged:
```
DEBUG Bearer token authentication successful
DEBUG OAuth authentication successful
```

### When to Use Hybrid Authentication

**Use Hybrid Mode When:**
- ✅ You have both human users (web UI) and service accounts (CI/CD)
- ✅ You want centralized authentication with OIDC
- ✅ You need service accounts with zero OAuth latency
- ✅ You want fine-grained RBAC for different actors

**Use Pure OAuth When:**
- ✅ Only human users access Scotty
- ✅ All access is via web UI or CLI with device flow
- ✅ No automation or service accounts needed

**Use Pure Bearer Mode When:**
- ✅ Only service accounts/automation access Scotty
- ✅ No human interactive access needed
- ✅ Simpler deployment without OAuth infrastructure

See [Configuration Documentation](configuration.html) for complete configuration reference and [config/README.md](../../config/README.md) for detailed security best practices.

## Frontend Integration

The Scotty frontend automatically detects OAuth mode and provides:

### Login Flow
- **Login page** shows "Continue with OAuth" button
- **OAuth callback page** handles the return from OIDC provider
- **User info component** displays authenticated user with logout option

### Token Management
- **Automatic token storage** in browser sessionStorage
- **Token validation** on each API request
- **Automatic logout** on token expiration or validation failure

## CLI Integration (scottyctl)

For CLI usage with OAuth-enabled Scotty, you have two options:

### Device Flow (Recommended)
```bash
# Use OAuth device flow for CLI authentication
scottyctl login --server http://localhost:21342
```

### Manual Token
```bash
# Extract token from browser sessionStorage and use manually
export SCOTTY_ACCESS_TOKEN=your_oauth_token
scottyctl --server http://localhost:21342 app:list
```

## Security Features

### PKCE (Proof Key for Code Exchange)
- **Enhanced security** for public clients (SPAs) - prevents authorization code interception attacks
- **SHA256 code challenge** - cryptographically secure random verifier with SHA256 hashing
- **Protected storage** - PKCE verifier stored in memory using `MaskedSecret` type (protected from memory dumps and logs)
- **Single-use verification** - verifier is removed from session store after successful token exchange

### CSRF Protection
- **State parameter validation** - combines session ID and CSRF token (`session_id:csrf_token` format)
- **Session-based tracking** - each OAuth flow gets a unique session with secure token storage
- **Automatic cleanup** - expired sessions removed every 5 minutes by background task
- **Time-limited sessions** - web flow sessions expire after 10 minutes, OAuth sessions after 5 minutes

### Token Security
- **Token validation on every request** - authentication middleware validates tokens before allowing access to protected endpoints
- **OIDC provider validation** - OAuth tokens validated against OIDC provider's userinfo endpoint
- **Session expiration** - OAuth sessions expire after 5 minutes of token exchange
- **Memory protection** - CSRF tokens and PKCE verifiers stored using `MaskedSecret` type with automatic zeroization
- **Secure storage** - tokens stored in browser sessionStorage (cleared when tab closes)

## Troubleshooting

### Common Issues

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

**PKCE Validation Failed**
```
Error: PKCE code challenge validation failed
```
- This indicates a potential security issue or session corruption
- Clear browser data and retry the authentication flow

**Session Expired**
```
Error: OAuth session not found or expired
```
- OAuth sessions have a limited lifetime
- Restart the authentication flow from the beginning

### Debug Commands

```bash
# Check Scotty configuration
curl http://localhost:21342/api/v1/info

# Test OAuth endpoints
curl -I http://localhost:21342/oauth/authorize

# Verify authentication (with valid token)
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
- **API Documentation**: http://localhost:21342/rapidoc
- **Health Check**: http://localhost:21342/api/v1/health (public)