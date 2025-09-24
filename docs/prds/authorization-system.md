# Product Requirements Document: Scotty Authorization System

## Executive Summary
Implement a lightweight, scope-based authorization system for Scotty that controls access to applications and their features, supporting both bearer token and OAuth authentication modes.

## Problem Statement
Currently, Scotty has all-or-nothing access control. Users with valid authentication can perform any action on any application. We need granular control to:
- Restrict sensitive operations (shell access, app deletion)
- Isolate applications by team/environment
- Support multi-tenant scenarios
- Enable safe read-only access for stakeholders

## Goals
1. **Security**: Prevent unauthorized access to sensitive operations
2. **Flexibility**: Support different access patterns without code changes
3. **Simplicity**: Easy to understand and manage permissions
4. **Performance**: Minimal impact on request latency
5. **Compatibility**: Work with existing auth modes (bearer, OAuth, dev)

## Non-Goals
- Full user management system
- Complex organizational hierarchies  
- Audit logging (separate feature)
- Row-level security within apps

## User Personas

### 1. Platform Administrator
- Manages Scotty infrastructure
- Creates app scopes and roles
- Assigns permissions globally
- Needs: Full control, ability to delegate

### 2. Development Team Lead
- Manages team's applications
- Grants access to team members
- Needs: Control over specific app scopes

### 3. Developer
- Deploys and manages applications
- Debugs via shell access
- Needs: Full access to dev/staging, limited production

### 4. Operations Engineer
- Monitors application health
- Restarts failed services
- Needs: View and manage, no shell or destroy

### 5. Stakeholder/Manager
- Views application status
- Tracks deployment progress
- Needs: Read-only access

## Core Concepts

### App Scopes
Collections of applications organized by purpose:
- **Environment-based**: production, staging, development
- **Team-based**: team-a, team-b, platform
- **Client-based**: client-x, client-y
- **Purpose-based**: databases, services, tools

Apps can belong to multiple scopes (e.g., an app could be in both "production" and "team-a" scopes).

### Permissions
Granular actions on applications:
- `view` - See app status and info
- `manage` - Start, stop, restart apps
- `logs` - View application logs
- `shell` - Execute shell commands in containers
- `create` - Create new apps in scope
- `destroy` - Delete apps from scope

### Roles
Named collections of permissions:
- `admin` - All permissions
- `developer` - All except destroy
- `operator` - View, manage, logs
- `viewer` - View only

### Assignments
Mapping of users/tokens to roles within scopes.

## User Stories

### Epic 1: Scope Management

**Story 1.1**: As an admin, I want to create app scopes
```yaml
Acceptance Criteria:
- Can create scope via API/CLI
- Scope has name and description
- Scopes are unique by name
- Changes persist across restarts
```

**Story 1.2**: As an admin, I want to assign apps to scopes
```yaml
Acceptance Criteria:
- Apps can declare scopes in .scotty.yml (single or multiple)
- Can specify scopes via CLI when creating/adopting apps
- Unassigned apps go to "default" scope
- Can reassign via API/CLI
- Scope assignment affects permissions immediately
- Apps can belong to multiple scopes simultaneously
```

### Epic 2: Role Management

**Story 2.1**: As an admin, I want to define custom roles
```yaml
Acceptance Criteria:
- Can create roles with specific permissions
- Can modify existing roles
- Built-in roles cannot be deleted
- Role changes apply immediately
```

### Epic 3: User Assignment

**Story 3.1**: As an admin, I want to assign roles to users
```yaml
Acceptance Criteria:
- Can assign by bearer token
- Can assign by OAuth email/subject
- Can assign different roles per scope
- Supports wildcard scope (*) for global roles
```

**Story 3.2**: As a developer, I want to know my permissions
```yaml
Acceptance Criteria:
- Can query current permissions via CLI
- Clear error messages when forbidden
- Can test permissions without performing actions
```

### Epic 4: Permission Enforcement

**Story 4.1**: As an admin, I want shell access restricted
```yaml
Acceptance Criteria:
- Only users with shell permission can access
- Applies per app scope
- Returns 403 Forbidden when denied
- Audit log shows attempts (future)
```

**Story 4.2**: As an ops engineer, I want to manage apps without shell
```yaml
Acceptance Criteria:
- Can start/stop/restart with manage permission
- Can view logs with logs permission
- Cannot access shell without permission
- Cannot delete apps without destroy permission
```

## Technical Requirements

### Performance
- Permission check < 5ms latency
- Support 10,000+ permission rules
- Cache permissions in memory
- No database required initially

### Storage
- File-based YAML for development
- Redis support for production
- Hot-reload configuration changes
- Backward compatible format

### Integration
- Middleware for all protected endpoints
- Works with existing auth modes
- Extends CurrentUser with permissions
- Compatible with WebSocket endpoints

### Security
- Deny by default
- No privilege escalation
- Secure token storage
- Protected management endpoints

## Implementation Phases

### Phase 1: Core Authorization ✅ **COMPLETED**
- [x] Casbin integration (v2.8 with proper RBAC model)
- [x] File-based YAML storage (config + policy files)
- [x] Authorization middleware with Permission enum
- [x] Scope and role models (Scopes, Roles, Assignments)
- [x] App scope assignment via .scotty.yml scopes field
- [x] Automatic scope sync during app discovery
- [x] Bearer token integration with authorization assignments
- [x] Direct user-scope-permission policy model
- [x] Comprehensive test suite with scope-based filtering

### Phase 2: Management API 🚧 **IN PROGRESS**
- [x] Core service methods (create_scope, assign_user_role, etc.)
- [ ] REST API endpoints for scope CRUD operations
- [ ] REST API endpoints for role management  
- [ ] REST API endpoints for user assignments
- [ ] Permission testing endpoint

### Phase 3: CLI Support
- [ ] scottyctl scope:* commands
- [ ] scottyctl role:* commands
- [ ] scottyctl auth:* commands
- [ ] Permission testing command

### Phase 4: Enforcement ✅ **COMPLETED**
- [x] App list filtering by View permission
- [x] API route protection with permission middleware
- [x] Comprehensive authorization tests
- [x] Middleware architecture fixes (State extractor, path extraction)
- [x] Bearer token authentication without legacy fallback
- [x] Permission debugging and error handling
- [ ] Shell access control (app:shell command)
- [ ] Destroy protection (enforced via existing middleware)
- [ ] Create restrictions (enforced via existing middleware)

### Phase 5: Production Features
- [ ] Redis adapter
- [ ] Performance optimization
- [ ] Migration tooling
- [ ] Documentation

## Current Implementation Details

### Architecture Overview
The authorization system is built on **Casbin RBAC** with the following key components:

#### Core Service (`AuthorizationService`)
- **Location**: `/scotty/src/services/authorization/` (modular structure)
- **Storage**: File-based YAML configuration + Casbin model file
- **Policy Model**: Direct user-scope-permission mapping for simplicity
- **Integration**: Automatic initialization and app scope synchronization
- **Debug Support**: Comprehensive debugging methods and proper permission reporting

#### Casbin Model
```
[request_definition]
r = sub, app, act

[policy_definition]  
p = sub, scope, act

[role_definition]
g = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = r.sub == p.sub && g(r.app, p.scope) && r.act == p.act
```

#### Permission Enum
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    View,    // See app status and info  
    Manage,  // Start, stop, restart apps
    Shell,   // Execute shell commands in containers
    Logs,    // View application logs
    Create,  // Create new apps in scope
    Destroy, // Delete apps from scope
}
```

### Bearer Token Integration
- **RBAC Only**: Looks up tokens exclusively in authorization assignments (`bearer:<token>`)
- **No Legacy Fallback**: Removed `api.access_token` fallback - all tokens must be explicitly assigned
- **User ID Format**: Uses `AuthorizationService::format_user_id()` for consistency
- **Token Validation**: `authorize_bearer_user()` only accepts tokens found in RBAC assignments

### App Scope Assignment
1. **Via .scotty.yml**: Apps declare `scopes: ["frontend", "staging"]` in settings
2. **Automatic Sync**: During app discovery, scopes are synced to Casbin policies
3. **Default Scope**: Apps without explicit scopes assigned to "default"
4. **Multiple Scopes**: Apps can belong to multiple scopes simultaneously

### API Protection
- **Middleware**: `require_permission(Permission::X)` on protected routes with proper State extractor
- **Path Extraction**: Updated to support full `/api/v1/authenticated/apps/{action}/{app_name}` paths
- **App List Filtering**: `/api/v1/authenticated/apps/list` only shows apps user can view
- **CurrentUser Integration**: Bearer tokens resolve to actual user identities
- **Error Handling**: Proper middleware ordering prevents "App state not found" errors

## Success Metrics
1. **Security**: Zero unauthorized access incidents
2. **Usability**: <2 min to grant new user access
3. **Performance**: <5ms permission check latency  
4. **Adoption**: 100% apps assigned to scopes
5. **Reliability**: 99.9% authorization service uptime

## Open Questions
1. Should we support permission inheritance between scopes?
2. How to handle emergency access scenarios?
3. Should permissions be time-limited?
4. Integration with external IdP scopes/roles?
5. Backup and disaster recovery for permissions?

## Appendix: Example Configuration

### Authorization Configuration (`config/casbin/policy.yaml`)

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

# Role definitions with permissions
roles:
  admin:
    description: "Full administrative access"
    permissions: ["*"]  # Wildcard for all permissions
    created_at: "2023-12-01T00:00:00Z"
  developer:
    description: "Full development access"
    permissions: ["view", "manage", "shell", "logs", "create"]
    created_at: "2023-12-01T00:00:00Z"
  operator:
    description: "Operations access without shell"
    permissions: ["view", "manage", "logs"]
    created_at: "2023-12-01T00:00:00Z"
  viewer:
    description: "Read-only access"
    permissions: ["view"]
    created_at: "2023-12-01T00:00:00Z"

# User/token assignments to roles within scopes
assignments:
  "bearer:frontend-dev-token":
    - role: "developer"
      scopes: ["frontend"]
  "bearer:backend-dev-token": 
    - role: "developer"
      scopes: ["backend"]
  "frontend-dev@example.com":
    - role: "developer" 
      scopes: ["frontend"]
  "ops-engineer@example.com":
    - role: "operator"
      scopes: ["frontend", "backend", "production"]
  "alice@example.com":
    - role: "admin"
      scopes: ["*"]  # Global admin access

# App -> Scope mappings (managed automatically from .scotty.yml)
apps:
  "my-frontend-app": ["frontend"]
  "my-backend-api": ["backend"] 
  "shared-service": ["frontend", "backend"]
  "prod-database": ["production"]
```

### App Configuration Example (`.scotty.yml`)

```yaml
# App declares which scopes it belongs to
scopes:
  - "frontend"
  - "staging"

public_services:
  - service: "web"
    port: 3000
    domains: []

environment:
  NODE_ENV: "development"
  
basic_auth: null
disallow_robots: true
time_to_live:
  Days: 7
```

## Remote Shell Feature (app:shell)

### Overview
The `app:shell` command enables secure remote shell access to Docker containers managed by Scotty, with authorization controls per app scope.

### Requirements

#### Functional Requirements
- Open interactive shell session to any service container
- Support multiple concurrent shell sessions
- Handle terminal resize events properly
- Forward signals (Ctrl+C, Ctrl+D, etc.)
- Support custom shell selection (sh, bash, etc.)

#### Security Requirements
- Require `shell` permission for app's scope
- Encrypt communication end-to-end
- Audit shell session initiation
- Terminate on permission revocation

#### Technical Requirements
- WebSocket-based communication
- PTY allocation for proper terminal emulation
- Compatible with existing auth modes
- Minimal latency (<50ms roundtrip)

### Implementation Approach
1. **WebSocket Protocol**: Extend existing WebSocket infrastructure
2. **Docker Integration**: Use docker exec with PTY allocation
3. **Terminal Emulation**: Handle via crossterm in scottyctl
4. **Authorization**: Check via Casbin before establishing connection

### User Stories

**Story S.1**: As a developer, I want to debug my application
```yaml
Acceptance Criteria:
- Can open shell with: scottyctl app:shell <app-name> [service]
- Defaults to first service if not specified
- Full terminal emulation (colors, cursor, etc.)
- Can run any command available in container
```

**Story S.2**: As an admin, I want to control shell access
```yaml
Acceptance Criteria:
- Only users with shell permission can connect
- Permission checked per app scope
- Failed attempts logged
- Active sessions can be monitored
```

### Example Usage
```bash
# Connect to default service
scottyctl app:shell my-app

# Connect to specific service
scottyctl app:shell my-app web

# With custom shell
scottyctl app:shell my-app --shell=/bin/bash

# Create app with scope assignment
scottyctl app:create my-app --scopes production,team-a

# Adopt existing app into scopes
scottyctl app:adopt existing-app --scopes staging,team-b
```