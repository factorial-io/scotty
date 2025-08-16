# Local OAuth Development & Testing Setup

## Overview

This document outlines how to develop and test OAuth integration locally with multiple approaches to accommodate different development workflows.

## Option 1: Full OAuth Stack (Recommended for Integration Testing)

### Prerequisites
1. GitLab OAuth application configured
2. Domain setup for consistent redirect URIs

### Setup
```bash
# 1. Create GitLab OAuth App at https://gitlab.com/-/profile/applications
# Name: Scotty Local Dev
# Redirect URI: http://scotty.local/oauth2/callback
# Scopes: read_user, read_api

# 2. Add to /etc/hosts (or use dnsmasq for *.local domains)
echo "127.0.0.1 scotty.local" | sudo tee -a /etc/hosts

# 3. Configure environment
cd examples/oauth2-proxy
cp .env.example .env
# Edit .env with your GitLab credentials

# 4. Start the full stack
docker-compose up -d

# 5. Build and run Scotty in development mode
cargo build
SCOTTY_API_HOST=0.0.0.0 SCOTTY_API_PORT=3000 ./target/debug/scotty &

# 6. Access at http://scotty.local
```

**Pros:** 
- Full OAuth flow testing
- Same as production behavior
- Tests cookie handling

**Cons:**
- Requires domain setup
- More complex debugging
- GitLab dependency

## Option 2: Development Mode with Auth Bypass (Recommended for Development)

Create a development mode that bypasses OAuth for faster iteration:

### Implementation
```rust
// In scotty/src/api/mod.rs - add development middleware
pub async fn auth_dev_bypass(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // In development, inject a fake user
    req.extensions_mut().insert(CurrentUser {
        email: "dev@localhost".to_string(),
        name: "Dev User".to_string(),
    });
    Ok(next.run(req).await)
}
```

### Usage
```bash
# Start Scotty in dev mode
SCOTTY_DEV_MODE=true cargo run --bin scotty

# Start frontend dev server
cd frontend
npm run dev

# Access at http://localhost:5173 (Vite dev server)
# or http://localhost:3000 (Scotty direct)
```

**Pros:**
- Fast development cycle
- No external dependencies
- Easy debugging

**Cons:**
- Doesn't test OAuth flow
- Different behavior than production

## Option 3: Hybrid Approach (Best of Both Worlds)

Support both modes with environment variable switching:

```bash
# Development mode - no OAuth
SCOTTY_AUTH_MODE=dev cargo run

# OAuth mode - full stack
SCOTTY_AUTH_MODE=oauth docker-compose -f docker-compose.dev.yml up
```

## Testing Strategy

### 1. Unit Tests
```bash
# Test OAuth token validation
cargo test auth

# Test API endpoints with mocked auth
cargo test api
```

### 2. Integration Tests
```bash
# Test full OAuth flow with test GitLab app
SCOTTY_TEST_MODE=oauth cargo test --test integration

# Test CLI device flow
cargo test --bin scottyctl test_device_flow
```

### 3. Manual Testing Checklist

**SPA Flow:**
- [ ] Redirect to GitLab when not authenticated
- [ ] Successful login redirects back to Scotty  
- [ ] API calls work with cookies
- [ ] Logout clears session
- [ ] Session persistence across browser refresh

**CLI Flow:**
- [ ] `scottyctl auth login` starts device flow
- [ ] Browser opens for GitLab authorization
- [ ] Tokens are stored securely
- [ ] API calls work with stored tokens
- [ ] Token refresh works automatically
- [ ] `scottyctl auth logout` clears tokens

## Recommended Development Workflow

1. **Initial Development:** Use Option 2 (dev bypass) for rapid iteration
2. **Feature Testing:** Switch to Option 1 for OAuth-specific features
3. **Integration Testing:** Use Option 3 with both modes
4. **Pre-commit:** Run full OAuth stack tests

## Configuration Files Structure

```
examples/
├── oauth2-proxy/           # Full OAuth stack
│   ├── docker-compose.yml
│   ├── .env.example
│   └── README.md
├── dev-mode/              # Development bypass
│   ├── docker-compose.dev.yml
│   └── scotty-dev.yaml
└── testing/               # Test configurations
    ├── test-gitlab-app.env
    └── integration-test.yml
```

## Environment Variables

```bash
# Core settings
SCOTTY_AUTH_MODE=dev|oauth           # Authentication mode
SCOTTY_DEV_MODE=true|false          # Enable development features
SCOTTY_API_HOST=0.0.0.0
SCOTTY_API_PORT=3000

# OAuth settings (when SCOTTY_AUTH_MODE=oauth)
GITLAB_CLIENT_ID=xxx
GITLAB_CLIENT_SECRET=xxx
COOKIE_SECRET=xxx
GITLAB_URL=https://gitlab.com       # Or your instance

# Development settings
SCOTTY_DEV_USER_EMAIL=dev@localhost
SCOTTY_DEV_USER_NAME=Dev User
```

## Next Steps

Which approach would you prefer to start with? I recommend beginning with Option 2 (dev bypass) to establish the basic OAuth infrastructure, then adding Option 1 for full testing.