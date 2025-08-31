# Authorization System

Scotty includes a powerful scope-based authorization system that controls access to applications and their features. This system allows you to restrict sensitive operations, isolate applications by team or environment, and support multi-tenant scenarios.

## Overview

The authorization system is built on **Casbin RBAC** and provides:

- **Scope-based access control**: Organize apps into logical scopes
- **Role-based permissions**: Define what actions users can perform
- **Flexible assignments**: Assign users to roles within specific scopes
- **Bearer token integration**: Secure API access with granular permissions
- **Automatic synchronization**: Apps declare scope membership via configuration

## Core Concepts

### App Scopes

Collections of applications organized by purpose:

- **Environment-based**: `production`, `staging`, `development`
- **Team-based**: `team-frontend`, `team-backend`, `platform`
- **Client-based**: `client-acme`, `client-widgets`
- **Purpose-based**: `databases`, `services`, `tools`

Apps can belong to multiple scopes simultaneously (e.g., an app could be in both `production` and `team-frontend` scopes).

### Permissions

Granular actions users can perform on applications:

- `view` - See app status and information
- `manage` - Start, stop, restart applications
- `logs` - View application logs  
- `shell` - Execute shell commands in containers
- `create` - Create new apps in scope
- `destroy` - Delete apps from scope

### Roles

Named collections of permissions for common access patterns:

- **`admin`** - All permissions (wildcard `*`)
- **`developer`** - Full access except destroy: `[view, manage, shell, logs, create]`
- **`operator`** - Operations without shell: `[view, manage, logs]`
- **`viewer`** - Read-only access: `[view]`

### Assignments

Map users or bearer tokens to roles within specific scopes:

```yaml
assignments:
  "frontend-dev@example.com":
    - role: "developer"
      scopes: ["frontend", "staging"]
  "bearer:dev-token":
    - role: "developer" 
      scopes: ["development"]
```

## Configuration

### Authorization Setup

Create `/config/casbin/policy.yaml`:

```yaml
# Scope definitions
scopes:
  frontend:
    description: "Frontend applications"
    created_at: "2023-12-01T00:00:00Z"
  backend:
    description: "Backend services"
    created_at: "2023-12-01T00:00:00Z"
  production:
    description: "Production environment"
    created_at: "2023-12-01T00:00:00Z"

# Role definitions
roles:
  admin:
    description: "Full administrative access"
    permissions: ["*"]
    created_at: "2023-12-01T00:00:00Z"
  developer:
    description: "Full development access"
    permissions: ["view", "manage", "shell", "logs", "create"]
    created_at: "2023-12-01T00:00:00Z"
  operator:
    description: "Operations access without shell"
    permissions: ["view", "manage", "logs"]
    created_at: "2023-12-01T00:00:00Z"

# User assignments
assignments:
  "frontend-dev@example.com":
    - role: "developer"
      scopes: ["frontend"]
  "backend-dev@example.com":
    - role: "developer"
      scopes: ["backend"]
  "ops@example.com":
    - role: "operator"
      scopes: ["frontend", "backend", "production"]
  "admin@example.com":
    - role: "admin"
      scopes: ["*"]  # Global access

# App scope mappings (managed automatically)
apps:
  "my-frontend-app": ["frontend"]
  "my-backend-api": ["backend"]
  "shared-service": ["frontend", "backend"]
```

### App Scope Assignment

Apps declare scope membership in their `.scotty.yml` file:

```yaml
# App belongs to frontend and staging scopes
scopes:
  - "frontend"
  - "staging"

public_services:
  - service: "web"
    port: 3000
    domains: []

environment:
  NODE_ENV: "development"
```

Apps without explicit scopes are assigned to the `default` scope.

## Authentication Integration

### Bearer Token Authentication

The authorization system requires explicit bearer token assignments:

1. **RBAC Only**: Only tokens explicitly assigned in authorization configuration are accepted
2. **No Legacy Fallback**: The `api.access_token` configuration is no longer used
3. **Token Format**: Bearer tokens are identified as `bearer:<token>` in assignments

```bash
# Using a bearer token with authorization (token must be in assignments)
curl -H "Authorization: Bearer admin" \
  https://scotty.example.com/api/v1/authenticated/apps/list
```

**Important**: Bearer tokens that are not explicitly listed in the `assignments` section will be rejected with a 401 Unauthorized response.

### OAuth Integration  

OAuth users are identified by their email address and can be assigned to roles:

```yaml
assignments:
  "alice@company.com":
    - role: "admin"
      scopes: ["*"]
  "bob@company.com":
    - role: "developer"
      scopes: ["team-frontend"]
```

## Permission Enforcement

### API Endpoints

All API endpoints are protected by permission checks:

- **App List**: `/api/v1/authenticated/apps/list` - Requires `view` permission, shows only accessible apps
- **App Management**: Start/stop/restart operations - Requires `manage` permission
- **Shell Access**: Future `app:shell` command - Requires `shell` permission
- **App Destruction**: Delete operations - Requires `destroy` permission

### Denied Access

When access is denied, users receive:

- **HTTP 403 Forbidden** responses
- **Clear error messages** explaining required permissions
- **Empty results** for list operations (apps they cannot access are hidden)

## Examples

### Multi-Team Setup

```yaml
# Teams with separate environments
scopes:
  team-alpha:
    description: "Team Alpha applications"
  team-beta:
    description: "Team Beta applications"
  production:
    description: "Production environment"

assignments:
  # Team Alpha developer
  "alice@company.com":
    - role: "developer"
      scopes: ["team-alpha"]
    - role: "viewer"
      scopes: ["production"]
      
  # Team Beta developer
  "bob@company.com":
    - role: "developer"
      scopes: ["team-beta"]
    - role: "viewer" 
      scopes: ["production"]
      
  # Platform engineer
  "charlie@company.com":
    - role: "operator"
      scopes: ["production"]
    - role: "admin"
      scopes: ["team-alpha", "team-beta"]
```

### Bearer Token Access

Configure bearer tokens in authorization assignments using their logical identifiers:

```yaml
assignments:
  # CI/CD deployment token (maps to bearer_tokens.deployment in API config)
  "identifier:deployment":
    - role: "developer"
      scopes: ["staging"]
      
  # Monitoring token (maps to bearer_tokens.monitoring in API config)
  "identifier:monitoring":
    - role: "viewer"
      scopes: ["production", "staging"]
      
  # Admin token (maps to bearer_tokens.admin in API config)
  "identifier:admin":
    - role: "admin"
      scopes: ["*"]
```

**Security Reminder**: The actual bearer tokens should be configured via environment variables:

```bash
# Set secure tokens via environment variables
export SCOTTY__API__BEARER_TOKENS__DEPLOYMENT="$(openssl rand -base64 32)"
export SCOTTY__API__BEARER_TOKENS__MONITORING="$(openssl rand -base64 32)" 
export SCOTTY__API__BEARER_TOKENS__ADMIN="$(openssl rand -base64 32)"
```

## Best Practices

### Security

1. **Principle of Least Privilege**: Grant minimum required permissions
2. **Scope Isolation**: Use scopes to separate sensitive environments
3. **Regular Audits**: Review assignments and remove unused access
4. **Emergency Access**: Maintain admin access for critical situations

### Organization

1. **Clear Naming**: Use descriptive scope and role names
2. **Documentation**: Document scope purposes and access patterns
3. **Consistency**: Establish naming conventions for scopes
4. **Automation**: Integrate with CI/CD for app scope assignment

### Performance

1. **Scope Structure**: Keep scope hierarchies simple
2. **Assignment Scope**: Avoid overly broad assignments
3. **Caching**: Authorization checks are cached for performance
4. **Regular Cleanup**: Remove obsolete scopes and assignments

## Migration

### Existing Installations

For existing Scotty installations:

1. **Breaking Change**: Bearer token authentication now requires RBAC assignments
2. **Migration Required**: Existing `api.access_token` must be added to assignments
3. **App Discovery**: Existing apps are assigned to `default` scope automatically
4. **OAuth Compatibility**: OAuth authentication continues to work unchanged

### Enabling Authorization

1. Create `/config/casbin/model.conf` and `/config/casbin/policy.yaml`
2. Define initial scopes, roles, and assignments
3. **Add existing bearer tokens to assignments** (if using bearer authentication)
4. Apps will automatically sync their scope memberships
5. API endpoints begin enforcing permissions immediately

**Migration Example**: If you currently use `api.access_token: "my-secret-token"`, follow these steps:

1. **Update API configuration** to use `api.bearer_tokens` with a logical identifier:
```yaml
api:
  bearer_tokens:
    legacy: "OVERRIDE_VIA_ENV_VAR"  # Will be overridden by environment variable
```

2. **Set the actual token via environment variable**:
```bash
export SCOTTY__API__BEARER_TOKENS__LEGACY="my-secret-token"
```

3. **Add the identifier to authorization policy.yaml**:
```yaml
assignments:
  "identifier:legacy":
    - role: "admin"
      scopes: ["*"]
```

**Recommended**: Generate a new secure token instead of reusing the old one:
```bash
export SCOTTY__API__BEARER_TOKENS__ADMIN="$(openssl rand -base64 32)"
```

**Warning**: The authorization system no longer falls back to legacy configuration. Missing token assignments will result in authentication failures.