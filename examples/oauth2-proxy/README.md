# OAuth2-Proxy Setup for Scotty

This configuration sets up oauth2-proxy with Traefik to handle GitLab OAuth authentication for the Scotty web interface.

## Setup Instructions

1. **Create GitLab OAuth Application**
   - Go to https://gitlab.com/-/profile/applications (or your GitLab instance)
   - Create a new application with these settings:
     - Name: "Scotty"
     - Redirect URI: `http://localhost/oauth2/callback` (adjust for your domain)
     - Scopes: `read_user`, `read_api`

2. **Configure Environment**
   ```bash
   cp .env.example .env
   # Edit .env with your GitLab OAuth credentials
   ```

3. **Generate Cookie Secret**
   ```bash
   openssl rand -base64 32 | tr -d "=" | tr "/" "_" | tr "+" "-"
   ```

4. **Start Services**
   ```bash
   docker-compose up -d
   ```

5. **Access Scotty**
   - Open http://localhost in your browser
   - You'll be redirected to GitLab for authentication
   - After successful login, you'll be redirected back to Scotty

## How It Works

- **Traefik** acts as a reverse proxy and routes requests
- **oauth2-proxy** handles the OAuth flow with GitLab
- All requests to Scotty are authenticated via the `oauth-auth` middleware
- The SPA receives user information through headers set by oauth2-proxy
- Cookies are used for session management (no more manual token input)

## Production Considerations

- Set `cookie-secure=true` when using HTTPS
- Use proper domain names instead of `localhost`
- Consider using environment-specific redirect URIs
- Store secrets securely (use Docker secrets, Kubernetes secrets, etc.)