# Scotty OAuth Setup with ForwardAuth

Production-ready OAuth authentication using oauth2-proxy with GitLab OIDC. Designed to protect multiple applications using reusable Traefik ForwardAuth middleware.

## Setup Instructions

### 1. Create GitLab OAuth Application
- Go to your GitLab instance: `https://gitlab.com/-/profile/applications` 
- Create a new application with:
  - **Name**: "Scotty"  
  - **Redirect URI**: `http://localhost/oauth2/callback`
  - **Scopes**: `openid`, `profile`, `email`

### 2. Configure Environment
```bash
# Copy example environment file
cp .env.example .env

# Edit .env with your GitLab OAuth credentials
# Or use 1Password: op run --env-file="./.env.1password" -- docker-compose up -d
```

### 3. Generate Cookie Secret
```bash
openssl rand -base64 32 | tr -d "=" | tr "/" "_" | tr "+" "-"
```

### 4. Start Services
```bash
docker-compose up -d
```

### 5. Access Scotty
- Open http://localhost
- You'll be redirected to GitLab for authentication  
- After login, you'll access Scotty dashboard

## Architecture

### ForwardAuth Pattern
- **Traefik** routes all requests and handles ForwardAuth
- **oauth2-proxy** validates authentication on every request
- **Scotty** receives requests with user headers set

### Session Management  
- **Redis-backed** sessions for better performance and scalability
- **Session expiry** of 24 hours with 5-minute refresh intervals
- **Large session support** for users with many GitLab groups
- **Session persistence** across container restarts
- **GitLab logout** invalidates session on next request
- **Manual logout**: Visit `http://localhost/oauth2/sign_out`

## Protecting Additional Apps

To protect other applications, simply add the ForwardAuth middleware:

```yaml
labels:
  - "traefik.http.routers.my-app.middlewares=oauth-auth@docker"
```

The `oauth-auth` middleware is reusable across all applications in the same Docker network.

## URLs
- **Application**: http://localhost
- **Traefik Dashboard**: http://localhost:8080  
- **OAuth Logout**: http://localhost/oauth2/sign_out

## Components

- **Traefik**: Reverse proxy with service discovery and ForwardAuth
- **oauth2-proxy**: GitLab OIDC authentication provider  
- **Redis**: Session storage for scalability and persistence
- **Scotty**: Micro-PaaS application (OAuth-protected)

## Production Notes
- Set `cookie-secure=true` for HTTPS
- Use proper domain names instead of localhost
- Store secrets securely (Docker secrets, etc.)
- Configure Redis persistence and backup
- Consider session timeout policies