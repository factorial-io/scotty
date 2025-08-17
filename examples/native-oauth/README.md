# Native OAuth Example

This example demonstrates Scotty's built-in OAuth authentication with OIDC provider integration.

## Overview

This setup shows how to configure Scotty with native OAuth support, eliminating the need for external authentication proxies like oauth2-proxy. Scotty handles the complete OAuth 2.0 Authorization Code flow with PKCE directly.

## Features

- **Native OAuth integration** - No external authentication proxy needed
- **OIDC provider support** - Works with GitLab, Auth0, Keycloak, Google, and other OIDC providers  
- **PKCE security** - Enhanced security for single-page applications
- **Session management** - Built-in session handling with CSRF protection
- **Frontend integration** - Complete SPA authentication flow

## Prerequisites

1. **OIDC Provider OAuth Application** - Create an OAuth app in your OIDC provider (GitLab, Auth0, etc.)
2. **Docker** - For running the example setup  
3. **OIDC credentials** - Client ID and secret from your OAuth app

## Setup Instructions

### 1. Create OIDC Provider OAuth Application

#### GitLab Example:
1. Go to GitLab → Settings → Applications
2. Create new application:
   - **Name**: Scotty Native OAuth Example
   - **Redirect URI**: `http://localhost:21342/api/oauth/callback`
   - **Scopes**: `openid`, `profile`, `email`, `read_user`
3. Save the **Application ID** and **Secret**

#### Other OIDC Providers:
- **Auth0**: Create application in Auth0 dashboard with redirect URI
- **Keycloak**: Create client in admin console
- **Google**: Configure OAuth 2.0 in Google Cloud Console

### 2. Configure Environment

Create `.env` file with your OAuth credentials:

```bash
# OIDC Provider OAuth Application credentials
OIDC_CLIENT_ID=your_oidc_application_id
OIDC_CLIENT_SECRET=your_oidc_application_secret

# OIDC Provider URL (examples below)
OIDC_ISSUER_URL=https://gitlab.com
# OIDC_ISSUER_URL=https://your-domain.auth0.com
# OIDC_ISSUER_URL=https://keycloak.example.com/auth/realms/your-realm

# OAuth callback URL (must match provider app configuration)
OAUTH_REDIRECT_URL=http://localhost:21342/api/oauth/callback
```

### 3. Run the Example

```bash
# Start Scotty with OAuth configuration
docker compose up -d

# Access the application
open http://localhost:21342
```

## Architecture

```
User Browser → Scotty Frontend → Scotty OAuth Endpoints → OIDC Provider
                    ↓
              localStorage tokens
                    ↓
           Authenticated API calls
```

### Authentication Flow

1. User clicks "Continue with OAuth" on login page
2. Frontend redirects to `/oauth/authorize`
3. Scotty generates PKCE challenge and redirects to OIDC provider
4. User authenticates with OIDC provider
5. OIDC provider redirects to `/api/oauth/callback` with authorization code
6. Scotty exchanges code for tokens using PKCE verifier and creates session
7. Frontend exchanges session for tokens and stores them in localStorage
8. Subsequent API calls use stored OAuth tokens

## Configuration

The example uses the following Scotty configuration:

```yaml
# config/oauth.yaml
api:
  bind_address: "0.0.0.0:21342"
  auth_mode: "oauth"
  oauth:
    oidc_issuer_url: "${OIDC_ISSUER_URL}"
    client_id: "${OIDC_CLIENT_ID}" 
    client_secret: "${OIDC_CLIENT_SECRET}"
    redirect_url: "${OAUTH_REDIRECT_URL}"
```

## Testing the Setup

### 1. Web Authentication

1. Visit http://localhost:21342
2. Click "Continue with OAuth" 
3. Authenticate with your OIDC provider
4. Verify you're redirected back and logged in
5. Check that your user info appears in the top-right corner

### 2. API Access

Test authenticated API endpoints:

```bash
# This should redirect to login (401)
curl -v http://localhost:21342/api/v1/authenticated/apps

# After getting token from browser localStorage:
curl -H "Authorization: Bearer YOUR_TOKEN" \
     http://localhost:21342/api/v1/authenticated/apps
```

### 3. CLI Integration

Use the device flow for CLI authentication:

```bash
# Run scottyctl login (uses device flow)
scottyctl login --server http://localhost:21342

# Or manually set token from browser
export SCOTTY_ACCESS_TOKEN=your_oauth_token
scottyctl --server http://localhost:21342 apps list
```

## Development vs Production

### Development 

This example is configured for local development:
- Uses `http://localhost` URLs
- Self-signed certificates acceptable
- Debug logging enabled

### Production Checklist

For production deployment:

1. **Use HTTPS**: Configure TLS certificates
2. **Update URLs**: Replace localhost with your domain
3. **Secure secrets**: Use secret management system
4. **Configure CORS**: Set appropriate CORS origins
5. **Enable logging**: Configure structured logging

### Example Production Config

```yaml
api:
  bind_address: "0.0.0.0:21342"
  auth_mode: "oauth"
  oauth:
    oidc_issuer_url: "https://gitlab.your-domain.com"
    client_id: "${OIDC_CLIENT_ID}"
    client_secret: "${OIDC_CLIENT_SECRET}" 
    redirect_url: "https://scotty.your-domain.com/api/oauth/callback"
```

## Troubleshooting

### Common Issues

**Redirect URI Mismatch**
```
Error: redirect_uri mismatch in OIDC provider
```
- Ensure OIDC provider OAuth app redirect URI exactly matches configuration
- Check protocol (http vs https) and port numbers
- Verify redirect URI is `http://localhost:21342/api/oauth/callback`

**Missing Environment Variables**
```
Error: Missing required OAuth configuration
```
- Verify `.env` file exists and contains all required variables
- Check environment variable names match expected format

**PKCE Validation Failed**
```
Error: PKCE code challenge validation failed  
```
- Clear browser data and restart authentication flow
- Verify session hasn't expired during OAuth flow

**Token Storage Issues**
```
Warning: Failed to store OAuth token
```
- Check browser localStorage is enabled
- Verify no browser extensions blocking localStorage

### Debug Commands

```bash
# Check container status
docker compose ps

# View Scotty logs
docker compose logs scotty

# Test OAuth endpoints
curl -I http://localhost:21342/oauth/authorize

# Check configuration
curl http://localhost:21342/api/v1/info
```

### Debug Mode

Enable debug logging:

```bash
# Add to docker-compose.yml environment
RUST_LOG=debug

# Or when running locally
RUST_LOG=debug cargo run --bin scotty
```

## Files in this Example

- `docker-compose.yml` - Container orchestration
- `config/oauth.yaml` - Scotty OAuth configuration  
- `.env.example` - Example environment variables
- `README.md` - This documentation

## Security Notes

1. **PKCE Protection**: Uses SHA256 PKCE for enhanced security
2. **CSRF Protection**: State parameter validates against session
3. **Token Security**: Tokens stored securely in browser localStorage  
4. **Session Management**: Built-in session cleanup and expiration
5. **Scope Limitation**: Only requests necessary OAuth scopes

## URLs and Access

- **Application**: http://localhost:21342
- **Login**: http://localhost:21342/login
- **OAuth Authorization**: http://localhost:21342/oauth/authorize
- **OAuth Callback**: http://localhost:21342/api/oauth/callback
- **OAuth Session Exchange**: http://localhost:21342/oauth/exchange
- **API Docs**: http://localhost:21342/rapidoc
- **Health**: http://localhost:21342/api/v1/health

For more information, see the [OAuth Authentication Guide](../../docs/content/oauth-authentication.md).