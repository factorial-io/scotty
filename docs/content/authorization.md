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

Granular actions users can perform on applications and the system:

**App-Specific Permissions** (require app context):
- `view` - See app status and information
- `manage` - Start, stop, restart applications
- `logs` - View application logs
- `shell` - Execute shell commands in containers and download files from them (`app:cp` from a container)
- `create` - Create new apps in scope, from any blueprint or from your own compose file
- `create:<blueprint>` - Create new apps in scope, but only from the named blueprint (e.g. `create:static-html`). A caller-supplied compose file, another blueprint, custom domains, caller-set public services, environment variables, a registry or notification receivers are rejected as forbidden: the blueprint decides the app's services, hostnames and environment, and the caller passes values through the blueprint's parameters. TTL, basic auth, the robots setting and allowlisted middlewares stay with the caller. `GET /api/v1/authenticated/blueprints` lists only the blueprints the user may create from.
- `destroy` - Delete apps from scope

Scotty refuses to start when a role lists an unknown or malformed permission (e.g. `create:` without a blueprint name). Scotty versions before templated blueprints read any unknown permission, including `create:<blueprint>`, as `*`. Don't use a policy with `create:<blueprint>` on an older Scotty.

Creating an app that already exists updates it. That also requires create rights in the existing app's scopes (with `create:<blueprint>`: only for apps created from that blueprint), and the requested scopes must equal the app's current scopes. Unlike other app permissions, rights in one of a multi-scope app's scopes are not enough: replacing the app needs create rights in each of its scopes. Only one create of an app runs at a time; a second one gets `409 Conflict` until the first has written the app. A re-create keeps the app's custom actions, and a create request can't bring its own: custom actions are added through their own endpoints and need approval.

**Custom Action Permissions** (for per-app custom actions):
- `action_read` - Execute read-only custom actions (no side effects)
- `action_write` - Execute custom actions that modify state
- `action_manage` - Create, list, and delete custom actions for apps in scope
- `action_approve` - Approve, reject, or revoke pending custom actions (admin-level)

**Global Permissions** (not tied to specific apps):
- `admin_read` - Read authorization configuration (scopes, roles, assignments, permissions). Also shows tasks without an app, and all tasks of the apps in the scopes where you hold it
- `admin_write` - Modify authorization configuration (create/update scopes, roles, assignments)

### Roles

Named collections of permissions for common access patterns:

- **`admin`** - All permissions (wildcard `*`)
- **`developer`** - Full access except destroy: `[view, manage, shell, logs, create, action_read, action_write, action_manage]`
- **`operator`** - Operations without shell: `[view, manage, logs, action_read]`
- **`viewer`** - Read-only access: `[view]`
- **`system_admin`** - Authorization management: `[admin_read, admin_write]`
- **`action_approver`** - Can approve/reject custom actions: `[view, action_approve]`

### Assignments

Map users or bearer tokens to roles within specific scopes:

```yaml
assignments:
  "frontend-dev@example.com":
    - role: "developer"
      scopes: ["frontend", "staging"]
  "identifier:ci-bot":   # key "ci-bot" in api.bearer_tokens
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
  developer:
    description: "Full development access"
    permissions: ["view", "manage", "shell", "logs", "create", "action_read", "action_write", "action_manage"]
  operator:
    description: "Operations access without shell"
    permissions: ["view", "manage", "logs", "action_read"]
  action_approver:
    description: "Can approve/reject custom actions"
    permissions: ["view", "action_approve"]
  system_admin:
    description: "System administration access"
    permissions: ["admin_read", "admin_write"]

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
  "system-admin@example.com":
    - role: "system_admin"
      scopes: ["*"]  # Can manage authorization but not apps

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
3. **Assignment Key**: Bearer tokens are assigned as `identifier:<name>`, where `<name>` is the token's key in `api.bearer_tokens` (never the token value itself)

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

**App Management Endpoints:**
- **App List**: `/api/v1/authenticated/apps/list` - Requires `view` permission, shows only accessible apps
- **App Management**: Start/stop/restart operations - Requires `manage` permission
- **Tasks**: `/api/v1/authenticated/tasks`, `/api/v1/authenticated/task/{uuid}`, task output streams and task update messages - You see a task of an app if you hold `admin_read` in one of the app's scopes, or the permission the task needs there: `view`, or for a custom action run the permission the action requires (`view` alone is not enough). Tasks without an app need `admin_read`. You always see tasks you started yourself (so you can follow an app you are creating before it is assigned to a scope). Other tasks answer like missing ones (404)
- **App Creation**: `/api/v1/authenticated/apps` (POST) - Requires `create` permission
- **Shell Access**: WebSocket shell sessions - Requires `shell` permission
- **Notifications**: `/api/v1/authenticated/apps/notify/add` and `/remove` - Require `manage` permission on the app
- **Custom Actions**: running an action requires `action_read` or `action_write` on the app, depending on the action
- **File Download**: `/api/v1/apps/{app}/services/{service}/files` (GET) - Requires `shell` permission
- **File Upload**: `/api/v1/apps/{app}/services/{service}/files` (PUT) - Requires `manage` permission
- **App Destruction**: Delete operations - Requires `destroy` permission

The complete list of operations and the permission each one requires is kept
in `scotty/tests/fixtures/authorization_matrix.yaml`. A test calls every
operation with every role and fails when the outcome differs, so that file
stays accurate.

**Admin API Endpoints** (require global permissions):
- **Scopes**: `/api/v1/authenticated/admin/scopes`
  - GET - Requires `admin_read` - List all scopes
  - POST - Requires `admin_write` - Create new scope
- **Roles**: `/api/v1/authenticated/admin/roles`
  - GET - Requires `admin_read` - List all roles
  - POST - Requires `admin_write` - Create new role
- **Assignments**: `/api/v1/authenticated/admin/assignments`
  - GET - Requires `admin_read` - List all assignments
  - POST - Requires `admin_write` - Create new assignment
  - DELETE - Requires `admin_write` - Remove assignment
- **Permissions**: `/api/v1/authenticated/admin/permissions`
  - GET - Requires `admin_read` - List available permissions
  - POST `/test` - Requires `admin_read` - Test permission for user/app combination
- **User Permissions**: `/api/v1/authenticated/admin/users/:user_id/permissions`
  - GET - Requires `admin_read` - Get permissions for specific user

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

### System Administration Example

Separate authorization management from app management:

```yaml
assignments:
  # System administrator - can manage authorization but NOT apps
  "auth-admin@example.com":
    - role: "system_admin"
      scopes: ["*"]

  # App administrator - can manage apps but NOT authorization
  "app-admin@example.com":
    - role: "admin"
      scopes: ["production", "staging"]
```

This separation allows you to:
- Grant authorization management (creating users/roles) without app access
- Grant app management without ability to change authorization
- Implement least privilege principle for admin roles

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
3. **Regular Cleanup**: Remove obsolete scopes and assignments

## Migration

### Existing Installations

For existing Scotty installations:

1. **Breaking Change**: Bearer token authentication now requires RBAC assignments
2. **Migration Required**: Existing `api.access_token` must be replaced by a bearer token in `api.bearer_tokens` with a matching `identifier:<name>` assignment
3. **App Discovery**: Existing apps are assigned to `default` scope automatically
4. **OAuth Compatibility**: OAuth authentication continues to work unchanged

### Enabling Authorization

1. Create `/config/casbin/policy.yaml` (`model.conf` ships with the repository and the Docker image; a missing `policy.yaml` is seeded from `policy.yaml.example`)
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
