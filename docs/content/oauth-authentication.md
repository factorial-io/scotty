# OAuth Authentication with oauth2-proxy and Traefik

Scotty supports OAuth authentication using oauth2-proxy and Traefik's ForwardAuth middleware. This setup provides GitLab OIDC-based authentication that protects your Scotty API endpoints while allowing public access to the web UI and health endpoints.

## Overview

Scotty supports three authentication modes configured via `auth_mode`:

- **`dev`**: Development mode with no authentication (uses fixed dev user)
- **`oauth`**: OAuth authentication via oauth2-proxy with GitLab OIDC  
- **`bearer`**: Traditional token-based authentication

In OAuth mode, authentication is handled by the `basic_auth.rs` middleware which extracts user information from headers set by oauth2-proxy.

## How OAuth Mode Works

### Architecture

```
User → Traefik → oauth2-proxy (ForwardAuth) → Scotty
                      ↓
                    Redis (sessions)
                      ↓  
                 GitLab OIDC
```

### Authentication Flow

1. **Public routes** (UI, health, assets) are accessible without authentication
2. **Protected routes** (`/api/v1/authenticated/*`) require ForwardAuth validation
3. **oauth2-proxy** validates user sessions and handles OIDC flows with GitLab
4. **Traefik** routes traffic and applies ForwardAuth middleware to protected endpoints
5. **Scotty** receives requests with authenticated user headers

### Route Protection

- **Public**: `/`, `/api/v1/health`, static assets, SPA routes
- **Protected**: `/api/v1/authenticated/*` - all API operations that modify state

## Setup Instructions

### 1. GitLab OAuth Application

1. Go to GitLab → Settings → Applications  
2. Create new application:
   - **Name**: Scotty  
   - **Redirect URI**: `http://localhost/oauth2/callback`
   - **Scopes**: `openid`, `profile`, `email`
3. Save the **Application ID** and **Secret**

### 2. Environment Configuration  

Create `.env` file with your OAuth credentials:

```bash
# GitLab OAuth Application credentials
GITLAB_CLIENT_ID=your_gitlab_application_id
GITLAB_CLIENT_SECRET=your_gitlab_application_secret

# Generate with: openssl rand -base64 32 | tr -d "=" | tr "/" "_" | tr "+" "-"
COOKIE_SECRET=your_random_32_character_string

# Optional: Custom GitLab instance URL (defaults to https://gitlab.com)
GITLAB_URL=https://your-gitlab.com

# OAuth callback URL (must match GitLab app configuration)
OAUTH_REDIRECT_URL=http://localhost/oauth2/callback
```

### 3. Scotty Configuration

Configure Scotty for OAuth mode in `config/local.yaml`:

```yaml
api:
  bind_address: "0.0.0.0:21342"
  auth_mode: "oauth"
  oauth_redirect_url: "/oauth2/start"
```

## Example Setup

Scotty includes a complete OAuth example in `examples/oauth2-proxy-oauth/`. 

### Quick Start

```bash
cd examples/oauth2-proxy-oauth

# Configure your GitLab OAuth credentials
cp .env.example .env
# Edit .env with your credentials

# Start the complete OAuth stack
docker compose up -d

# Access Scotty
open http://localhost
```

### What's Included

The example provides:

- **Traefik**: Reverse proxy with ForwardAuth middleware
- **oauth2-proxy**: GitLab OIDC authentication handler  
- **Redis**: Session storage for scalability
- **Scotty**: Configured in OAuth mode

## Docker Compose Configuration

Here's the key configuration from the working example:

```yaml
services:
  # Traefik with ForwardAuth setup
  traefik:
    image: traefik:v3.0
    command:
      - "--providers.docker=true"
      - "--providers.docker.exposedbydefault=false"
      - "--entrypoints.web.address=:80"
    ports:
      - "80:80"
      - "8080:8080"  # Dashboard
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro

  # oauth2-proxy for GitLab OIDC
  oauth2-proxy:
    image: quay.io/oauth2-proxy/oauth2-proxy:v7.6.0
    command:
      - --provider=gitlab
      - --client-id=${GITLAB_CLIENT_ID}
      - --client-secret=${GITLAB_CLIENT_SECRET}
      - --cookie-secret=${COOKIE_SECRET}
      - --redirect-url=${OAUTH_REDIRECT_URL}
      - --oidc-issuer-url=${GITLAB_URL:-https://gitlab.com}
      - --pass-user-headers=true
      - --pass-access-token=true
      - --session-store-type=redis
      - --redis-connection-url=redis://redis:6379
    labels:
      # OAuth2-proxy routes  
      - "traefik.http.routers.oauth.rule=Host(`localhost`) && PathPrefix(`/oauth2`)"
      # Reusable ForwardAuth middleware
      - "traefik.http.middlewares.oauth-auth.forwardauth.address=http://oauth2-proxy:4180/oauth2/auth"
      - "traefik.http.middlewares.oauth-auth.forwardauth.trustForwardHeader=true"
      - "traefik.http.middlewares.oauth-auth.forwardauth.authResponseHeaders=X-Auth-Request-User,X-Auth-Request-Email,X-Auth-Request-Access-Token"

  # Scotty with route-based authentication
  scotty:
    environment:
      - SCOTTY__API__AUTH_MODE=oauth
      - SCOTTY__API__OAUTH_REDIRECT_URL=/oauth2/start
    labels:
      # Protected API endpoints (require OAuth)
      - "traefik.http.routers.scotty-authenticated.rule=Host(`localhost`) && PathPrefix(`/api/v1/authenticated/`)"
      - "traefik.http.routers.scotty-authenticated.middlewares=oauth-auth@docker"
      - "traefik.http.routers.scotty-authenticated.priority=100"
      # Public endpoints (no authentication)
      - "traefik.http.routers.scotty-public.rule=Host(`localhost`) && !PathPrefix(`/oauth2`)"
      - "traefik.http.routers.scotty-public.priority=50"

  # Redis for session storage
  redis:
    image: redis:7-alpine
    command: redis-server --appendonly yes
    volumes:
      - redis-data:/data
```

## User Information

When authenticated, oauth2-proxy provides these headers to Scotty:

- **`X-Auth-Request-User`**: GitLab username
- **`X-Auth-Request-Email`**: User's email address
- **`X-Auth-Request-Access-Token`**: GitLab OAuth access token

Scotty's authentication middleware extracts this information and creates a `CurrentUser` object available to all handlers.

## Development vs Production

### Development Setup

Use the provided example for local development:

```bash
cd examples/oauth2-proxy-dev    # No auth required
# or
cd examples/oauth2-proxy-oauth  # Full OAuth setup
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

1. **Use HTTPS**: Configure TLS in Traefik and set `--cookie-secure=true`
2. **Proper domains**: Replace `localhost` with your actual domain
3. **Secure secrets**: Use Docker secrets or external secret management
4. **Session persistence**: Configure Redis persistence and backup
5. **Security headers**: Add additional security middleware

## Session Management

- **Redis-backed sessions** for scalability and persistence
- **24-hour expiry** with 5-minute refresh intervals  
- **Session persistence** across container restarts
- **Manual logout**: Visit `http://localhost/oauth2/sign_out`
- **GitLab logout** invalidates session on next request

## Protecting Additional Applications

The ForwardAuth middleware is reusable. To protect other applications:

```yaml
labels:
  - "traefik.http.routers.my-app.middlewares=oauth-auth@docker"
```

## Troubleshooting

### Common Issues

**Redirect URI Mismatch**
```
Error: redirect_uri mismatch in GitLab
```
- Ensure GitLab OAuth app redirect URI matches `OAUTH_REDIRECT_URL`
- Check for trailing slashes and exact URL matching

**Missing OAuth Headers**
```
Warning: Missing OAuth headers from proxy
```
- Verify oauth2-proxy is running and healthy
- Check Traefik ForwardAuth middleware configuration
- Ensure `authResponseHeaders` are configured correctly

**Session/Cookie Issues**
```  
Error: Invalid cookie or session expired
```
- Clear browser cookies and retry
- Verify `COOKIE_SECRET` is set and consistent
- Check Redis connectivity and session storage

### Debug Commands

```bash
# Check service status
docker compose ps

# View oauth2-proxy logs  
docker compose logs oauth2-proxy

# View Traefik configuration
curl http://localhost:8080/api/rawdata

# Test authentication flow
curl -v http://localhost/api/v1/authenticated/apps
```

## URLs and Access

- **Application**: http://localhost
- **Traefik Dashboard**: http://localhost:8080
- **OAuth Logout**: http://localhost/oauth2/sign_out
- **Health Check**: http://localhost/api/v1/health (public)

## Security Notes  

1. **Route-based protection**: Only `/api/v1/authenticated/*` requires authentication
2. **Header validation**: User information comes from trusted oauth2-proxy headers
3. **Session security**: Redis-backed sessions with configurable expiry
4. **Access control**: Configure appropriate GitLab OAuth scopes
5. **Network isolation**: Use dedicated Docker networks for security

For complete working examples, see the `examples/oauth2-proxy-oauth/` and `examples/oauth2-proxy-dev/` directories in the Scotty repository.