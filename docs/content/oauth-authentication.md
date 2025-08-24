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
```

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

This information is available to both the frontend (stored in localStorage) and backend (through authentication middleware).

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

## Frontend Integration

The Scotty frontend automatically detects OAuth mode and provides:

### Login Flow
- **Login page** shows "Continue with OAuth" button
- **OAuth callback page** handles the return from OIDC provider
- **User info component** displays authenticated user with logout option

### Token Management
- **Automatic token storage** in browser localStorage
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
# Extract token from browser localStorage and use manually  
export SCOTTY_ACCESS_TOKEN=your_oauth_token
scottyctl --server http://localhost:21342 app:list
```

## Security Features

### PKCE (Proof Key for Code Exchange)
- **Enhanced security** for public clients (SPAs)
- **Code challenge/verifier** prevents code interception attacks
- **SHA256 hashing** of random code verifier

### CSRF Protection  
- **State parameter** validation prevents CSRF attacks
- **Session-based** state tracking
- **Automatic cleanup** of expired sessions

### Token Security
- **Short-lived tokens** with appropriate expiration
- **Secure storage** recommendations for production
- **Token validation** on each authenticated request

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

## Migration from oauth2-proxy

If you're migrating from the previous oauth2-proxy setup:

1. **Remove external dependencies**: No need for Traefik ForwardAuth or oauth2-proxy containers
2. **Update configuration**: Switch from proxy-based to native OAuth configuration  
3. **Update redirect URLs**: Change from `/oauth2/callback` to `/api/oauth/callback`
4. **Update configuration keys**: Change `gitlab_url` to `oidc_issuer_url`
5. **Test authentication flow**: Verify the complete OAuth flow works end-to-end

The native OAuth implementation provides better integration, reduced complexity, and enhanced security while maintaining the same user experience.