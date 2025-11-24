# Configuration Guide

This directory contains configuration files for Scotty. Configuration is loaded in layers, with later sources overriding earlier ones.

## Configuration Loading Order

1. **`default.yaml`** - Base configuration (deployment-specific, not in git by default)
2. **`local.yaml`** - Local overrides (git-ignored)
3. **Environment variables** - Highest priority, use `SCOTTY__` prefix

Example:
```bash
# Override bearer token via environment variable
export SCOTTY__API__BEARER_TOKENS__ADMIN="super-secret-token-abc123"

# Override registry password
export SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD="registry-password"
```

## Getting Started

### 1. Create Your Configuration

Copy the example files to create your configuration:

```bash
cp config/default.yaml.example config/default.yaml
cp config/casbin/policy.yaml.example config/casbin/policy.yaml
```

### 2. Configure Authentication

Edit `config/default.yaml` and set your authentication mode:

**Option A: Development Mode (No Authentication)**
```yaml
api:
  auth_mode: development
```

**Option B: Bearer Token Mode**
```yaml
api:
  auth_mode: bearer
  bearer_tokens:
    admin: "your-random-secure-token-here"
    ci-bot: "another-random-secure-token"
```

**Option C: OAuth Mode (with Bearer Token Fallback)**
```yaml
api:
  auth_mode: oauth
  oauth:
    oidc_issuer_url: "https://your-oidc-provider.example.com"
  bearer_tokens:
    ci-bot: "token-for-service-accounts"
```

### 3. Configure RBAC (Authorization)

Edit `config/casbin/policy.yaml` to define:
- **Scopes**: Logical groupings (e.g., production, staging, client-a)
- **Roles**: Permission sets (viewer, developer, operator, admin)
- **Assignments**: Map users/service accounts to roles + scopes

## Understanding Identifiers vs Tokens

**CRITICAL SECURITY CONCEPT:**

- **Identifiers** are NAMES used in `policy.yaml` for RBAC assignments
- **Token VALUES** are secrets stored in `default.yaml` or environment variables
- The `bearer_tokens` configuration maps identifiers to actual tokens

### Example:

**In `default.yaml` (or via environment variable):**
```yaml
api:
  bearer_tokens:
    ci-bot: "actual-secret-token-abc123"
```

**In `policy.yaml`:**
```yaml
assignments:
  identifier:ci-bot:
  - role: developer
    scopes:
    - staging
```

**When a request arrives:**
1. Client sends: `Authorization: Bearer actual-secret-token-abc123`
2. Scotty looks up token in `bearer_tokens` → finds identifier `ci-bot`
3. Scotty checks `policy.yaml` → `identifier:ci-bot:` has role `developer` in scope `staging`
4. Authorization decision made based on permissions

### Naming Convention:

- **OAuth users**: Use email addresses from OIDC tokens (e.g., `admin@example.com`)
- **Service accounts**: Use `identifier:` prefix with semantic names (e.g., `identifier:ci-bot:`)
- **Avoid**: Don't use token values or confusing names like `identifier:test-bearer-token-123:`

## Production Deployment

### Security Best Practices

1. **NEVER commit secrets to git**
   - Keep `default.yaml` out of git (or use placeholders)
   - Keep `local.yaml` out of git (already ignored)
   - Use environment variables for all secrets

2. **Use environment variables for sensitive values**
   ```bash
   # Bearer tokens
   SCOTTY__API__BEARER_TOKENS__ADMIN=secure-random-token
   SCOTTY__API__BEARER_TOKENS__CI_BOT=another-secure-token

   # OAuth secrets
   SCOTTY__API__OAUTH__CLIENT_SECRET=oauth-client-secret

   # Registry passwords
   SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD=registry-password
   ```

3. **Generate secure random tokens**
   ```bash
   # Generate a secure random token
   openssl rand -base64 32
   ```

### Docker Deployment

The Docker image **does NOT include** `default.yaml` or `policy.yaml` to prevent accidentally baking secrets into images.

**Option 1: Mount configuration directory (recommended)**
```bash
docker run -d \
  -v /path/to/your/config:/app/config:ro \
  -p 21342:21342 \
  scotty:latest
```

**Option 2: Use environment variables**
```bash
docker run -d \
  -e SCOTTY__API__AUTH_MODE=bearer \
  -e SCOTTY__API__BEARER_TOKENS__ADMIN=your-secure-token \
  -e SCOTTY__APPS__DOMAIN_SUFFIX=your-domain.site \
  -e SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD=registry-pwd \
  -p 21342:21342 \
  scotty:latest
```

**Option 3: Hybrid approach**
```bash
# Mount policy.yaml (RBAC config, no secrets)
# Use env vars for secrets
docker run -d \
  -v /path/to/config/casbin:/app/config/casbin:ro \
  -v /path/to/config/blueprints:/app/config/blueprints:ro \
  -e SCOTTY__API__AUTH_MODE=bearer \
  -e SCOTTY__API__BEARER_TOKENS__ADMIN=your-secure-token \
  -p 21342:21342 \
  scotty:latest
```

### Docker Compose Example

```yaml
services:
  scotty:
    image: scotty:latest
    ports:
      - "21342:21342"
    volumes:
      # Mount configuration directory (read-only)
      - ./config:/app/config:ro
      # Mount apps directory
      - ./apps:/app/apps
      # Mount Docker socket for container management
      - /var/run/docker.sock:/var/run/docker.sock
    environment:
      # Override secrets via environment variables
      - SCOTTY__API__BEARER_TOKENS__ADMIN=${ADMIN_TOKEN}
      - SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD=${REGISTRY_PASSWORD}
    restart: unless-stopped
```

Store secrets in a `.env` file (git-ignored):
```bash
ADMIN_TOKEN=your-secure-token-here
REGISTRY_PASSWORD=your-registry-password
```

## File Structure

```
config/
├── README.md                      # This file
├── default.yaml.example           # Template configuration (commit to git)
├── default.yaml                   # Your configuration (git-ignored)
├── local.yaml                     # Local overrides (git-ignored)
├── casbin/
│   ├── model.conf                 # RBAC model (safe to commit)
│   ├── policy.yaml.example        # Template RBAC policy (commit to git)
│   └── policy.yaml                # Your RBAC policy (can commit if no secrets)
└── blueprints/
    ├── drupal-lagoon.yaml         # App blueprints (safe to commit)
    └── nginx-lagoon.yaml
```

## Troubleshooting

### "Authentication failed" errors

1. Check that your token is correctly configured:
   ```bash
   # Via environment variable
   export SCOTTY__API__BEARER_TOKENS__ADMIN="your-token"

   # Or in config/default.yaml
   api:
     bearer_tokens:
       admin: "your-token"
   ```

2. Verify the identifier exists in `policy.yaml`:
   ```yaml
   assignments:
     identifier:admin:
     - role: admin
       scopes:
       - '*'
   ```

3. Test your token:
   ```bash
   curl -H "Authorization: Bearer your-token" \
     http://localhost:21342/api/v1/health
   ```

### "Permission denied" errors

1. Check your role assignments in `policy.yaml`
2. Verify the user/identifier has the required permission for the scope
3. Use the admin API to inspect assignments:
   ```bash
   scottyctl admin:assignments:list
   ```

### Configuration not loading

1. Check file locations and permissions
2. Verify YAML syntax (use `yamllint` or online validator)
3. Check Scotty logs for configuration errors:
   ```bash
   RUST_LOG=debug scotty
   ```

## See Also

- [Main README](../README.md) - Project overview
- [Authentication Documentation](../docs/authentication.md) - Detailed auth guide
- [Authorization Documentation](../docs/authorization.md) - RBAC details
