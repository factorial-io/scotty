# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scotty is a micro Platform-as-a-Service (PaaS) for managing Docker Compose-based applications. The codebase consists of:

- **scotty**: HTTP server providing REST API and WebSocket support for managing Docker Compose applications
- **scottyctl**: CLI application for interacting with the scotty server
- **scotty-core**: Shared business logic and utilities
- **scotty-types**: Shared type definitions (TypeScript-compatible via ts-rs)
- **frontend**: SvelteKit-based web interface (tightly coupled with the API - no backwards compatibility required)

## Development Commands

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test with output
cargo test test_name -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test test_name -p scotty -- --nocapture
```

### Running the Server
```bash
# Development mode (no authentication required)
SCOTTY__API__AUTH_MODE=dev cargo run --bin scotty

# Or use .env file for local configuration (recommended)
# Create a .env file with:
#   SCOTTY__API__AUTH_MODE=dev
#   SCOTTY__DOCKER__REGISTRIES__MYREGISTRY__PASSWORD=secret
cargo run --bin scotty

# With info logging
RUST_LOG=info cargo run --bin scotty

# With telemetry enabled
SCOTTY__TELEMETRY=metrics,traces cargo run --bin scotty

# View current configuration
cargo run --bin scotty -- config

# Choose telemetry transport(s) - can enable one or both (default: telemetry-grpc)
cargo build                                                            # gRPC only (default)
cargo build --no-default-features --features telemetry-http            # HTTP only
cargo build --no-default-features --features telemetry-grpc,telemetry-http  # Both (runtime config)
```

### Running scottyctl
```bash
# Basic command structure
cargo run --bin scottyctl -- <command>

# With server and auth (via command-line args)
cargo run --bin scottyctl -- --server http://localhost:21342 --access-token <token> app:list

# With server and auth (via environment variables)
export SCOTTY_SERVER=http://localhost:21342
export SCOTTY_ACCESS_TOKEN=<token>
cargo run --bin scottyctl -- app:list

# When server is running in dev mode (SCOTTY__API__AUTH_MODE=dev), no token needed
cargo run --bin scottyctl -- app:list
```

### Frontend Development
```bash
cd frontend
npm install
npm run dev        # Development server
npm run build      # Production build
npm run check      # Type checking
```

### Prerequisites
Start Traefik (required for local development):
```bash
cd apps/traefik
docker-compose up -d
```

## Architecture

### Workspace Structure

The project uses a Cargo workspace with the following members:
- `scotty`: Main server binary and API implementation
- `scottyctl`: CLI client
- `scotty-core`: Shared business logic (Docker operations, settings, tasks)
- `scotty-types`: Type definitions (uses ts-rs for TypeScript generation)
- `ts-generator`: Utility for generating TypeScript bindings

### Scotty Server Architecture

**Entry Point**: `scotty/src/main.rs`
- Initializes `AppState` with settings, Docker client, task manager
- Sets up telemetry (OpenTelemetry for metrics and tracing)
- Spawns HTTP server, Docker integration, and background tasks

**Key Modules**:
- `api/`: HTTP API layer
  - `router.rs`: Axum router with OpenAPI documentation (utoipa)
  - `rest/handlers/`: REST endpoint handlers organized by domain (apps, admin, tasks, etc.)
  - `websocket/`: WebSocket handlers for real-time features (logs, shell, task output)
  - `middleware/`: Authorization middleware using Casbin RBAC
  - `rate_limiting/`: Rate limiting per authentication tier
- `docker/`: Core Docker Compose orchestration
  - `state_machine_handlers/`: Task execution steps (create directory, run compose, etc.)
  - `services/`: Long-running services (log streaming, shell sessions)
  - `loadbalancer/`: Traefik/HAProxy configuration generation
- `services/authorization/`: Casbin-based RBAC with scopes, roles, and permissions
- `oauth/`: OAuth 2.0 device flow and web flow implementation
- `tasks/`: Task execution and output streaming
- `notification/`: Webhook, Mattermost, GitLab notifications

**State Management**:
- `AppState` (shared via `Arc`) contains:
  - Settings (loaded from config files and env vars)
  - Docker client (Bollard)
  - Task manager for async operations
  - Authorization service
  - Metrics collectors

### Authorization System

Uses Casbin for RBAC with three concepts:
- **Scopes**: Logical groupings (e.g., `client-a`, `qa`, `default`)
- **Roles**: Permission sets (e.g., `admin`, `developer`, `viewer`)
- **Assignments**: Map users to roles+scopes

**Configuration**: `config/casbin/policy.yaml`

**Available Permissions**: `view`, `manage`, `create`, `destroy`, `shell`, `logs`, `admin_read`, `admin_write`, `action_read`, `action_write`, `action_manage`, `action_approve`

#### Assignment Types

Scotty supports three types of user assignments with a specific precedence order:

1. **Exact email match** (highest priority)
   - Syntax: `user@factorial.io`
   - Matches specific user email (case-insensitive per RFC 5321)
   - Use for: Individual users requiring specific permissions

2. **Domain pattern match** (fallback)
   - Syntax: `@factorial.io`
   - Matches all users from a specific email domain
   - Use for: Granting consistent permissions to all users from an organization
   - Validation rules:
     - Must start with `@`
     - Must contain at least one dot (e.g., `@factorial.io`)
     - Cannot contain additional `@` symbols
   - Security: Prevents subdomain attacks (`user@evil.factorial.io` does NOT match `@factorial.io`)

3. **Wildcard match** (baseline)
   - Syntax: `*`
   - Matches all users regardless of identity
   - Use for: Default baseline permissions for anonymous or unassigned users
   - Always additive (combined with exact/domain matches)

#### Assignment Precedence

When a user authenticates, Scotty resolves their permissions using this precedence:

1. **Exact email match** - If found, use these assignments
2. **Domain match** - If no exact match, check for domain pattern (e.g., `@factorial.io`)
3. **Wildcard** - Always added to all users (additive, not exclusive)

**Example**: User `developer@factorial.io` authenticates:
- Has exact assignment: Uses exact + wildcard assignments
- No exact, has domain `@factorial.io`: Uses domain + wildcard assignments
- No exact, no domain: Uses only wildcard assignments

#### Configuration Example

```yaml
# config/casbin/policy.yaml
scopes:
  client-a:
    description: Client A Production
  qa:
    description: QA Environment
  default:
    description: Default scope for public apps

roles:
  admin:
    permissions: ['*']
    description: Full system access
  developer:
    permissions: ['view', 'manage', 'create', 'shell', 'logs']
    description: Developer access (no destroy)
  viewer:
    permissions: ['view']
    description: Read-only access

assignments:
  # Exact email - highest priority
  stephan@factorial.io:
    - role: admin
      scopes: ['*']  # Access to all scopes

  # Domain pattern - applies to all @factorial.io users
  # Only used if no exact email match exists
  '@factorial.io':
    - role: developer
      scopes: ['client-a', 'qa']

  # Wildcard - baseline for everyone
  # Always combined with exact/domain assignments
  '*':
    - role: viewer
      scopes: ['default']
```

#### Use Cases

**Individual Admin Access**:
```yaml
stephan@factorial.io:
  - role: admin
    scopes: ['*']
```

**Organization-Wide Developer Access**:
```yaml
'@factorial.io':
  - role: developer
    scopes: ['client-a', 'qa', 'staging']
```

**Public Read-Only Access**:
```yaml
'*':
  - role: viewer
    scopes: ['public-demos']
```

**Mixed Access Levels**:
```yaml
# Admin gets special access
admin@factorial.io:
  - role: admin
    scopes: ['production']

# All other @factorial.io users get developer
'@factorial.io':
  - role: developer
    scopes: ['staging', 'qa']

# Everyone gets viewer access to demos
'*':
  - role: viewer
    scopes: ['demos']
```

#### Implementation Details

- **Custom Casbin matcher**: Uses `user_match()` function for domain/wildcard matching
- **Case-insensitive**: Email matching follows RFC 5321 (case-insensitive)
- **Security**: Domain patterns validated to prevent attacks
- **Location**: `scotty/src/services/authorization/casbin.rs`
- **Tests**: `scotty/tests/authorization_domain_test.rs`

### Custom Actions

Custom actions allow users to define and execute arbitrary commands on app services. They support an approval workflow for security control.

#### Permission Model

Custom actions use four dedicated permissions:

| Permission | Description |
|------------|-------------|
| `action_read` | Execute read-only actions (no side effects) |
| `action_write` | Execute actions that modify state |
| `action_manage` | Create, list, and delete custom actions for apps in user's scope |
| `action_approve` | Approve/reject pending actions (admin-level) |

#### Action Status Workflow

Actions go through an approval workflow:

```
┌─────────┐     approve      ┌──────────┐
│ Pending │ ───────────────► │ Approved │ ◄─── Can be executed
└─────────┘                  └──────────┘
     │                            │
     │ reject                     │ revoke
     ▼                            ▼
┌──────────┐                ┌─────────┐
│ Rejected │                │ Revoked │
└──────────┘                └─────────┘

Actions can also expire if a TTL is configured → Expired
```

Only **Approved** actions can be executed.

#### CLI Commands

**Creating a custom action:**
```bash
# Create an action that requires action_write permission
scottyctl action:create my-app deploy-db \
  --description "Run database migrations" \
  --permission action_write \
  --command "web:php artisan migrate" \
  --command "worker:php artisan queue:restart"

# Create a read-only action (action_read permission)
scottyctl action:create my-app check-status \
  --description "Check application health" \
  --permission action_read \
  --command "web:php artisan health:check"
```

**Listing actions:**
```bash
# List all custom actions for an app
scottyctl action:list my-app
```

**Getting action details:**
```bash
# View full details of an action
scottyctl action:get my-app deploy-db
```

**Running an action:**
```bash
# Execute an approved action
scottyctl app:action my-app deploy-db
```

**Deleting an action:**
```bash
# Remove a custom action
scottyctl action:delete my-app deploy-db
```

#### Admin Commands

Users with `action_approve` permission can manage the approval workflow:

```bash
# List all pending actions across all apps
scottyctl admin:actions:pending

# View details of a pending action
scottyctl admin:actions:get my-app deploy-db

# Approve an action (with optional comment)
scottyctl admin:actions:approve my-app deploy-db --comment "Reviewed and approved"

# Reject an action
scottyctl admin:actions:reject my-app deploy-db --comment "Security concern with command"

# Revoke a previously approved action
scottyctl admin:actions:revoke my-app deploy-db --comment "No longer needed"
```

#### Role Configuration

To grant custom action permissions, update `config/casbin/policy.yaml`:

```yaml
roles:
  # Developers can manage and execute actions
  developer:
    permissions: ['view', 'manage', 'action_read', 'action_write', 'action_manage']
    description: Developer with action management

  # Action approvers (security team)
  action_approver:
    permissions: ['view', 'action_approve']
    description: Can approve/reject custom actions

  # Full admin
  admin:
    permissions: ['*']
    description: Full access including action approval
```

#### API Endpoints

| Method | Endpoint | Permission | Description |
|--------|----------|------------|-------------|
| `POST` | `/api/v1/authenticated/apps/{app}/custom-actions` | `action_manage` | Create action |
| `GET` | `/api/v1/authenticated/apps/{app}/custom-actions` | `action_manage` | List actions |
| `GET` | `/api/v1/authenticated/apps/{app}/custom-actions/{name}` | `action_manage` | Get action details |
| `DELETE` | `/api/v1/authenticated/apps/{app}/custom-actions/{name}` | `action_manage` | Delete action |
| `POST` | `/api/v1/authenticated/apps/{app}/actions` | `action_read` or `action_write` | Execute action |
| `GET` | `/api/v1/authenticated/admin/actions/pending` | `action_approve` | List pending |
| `POST` | `/api/v1/authenticated/admin/actions/{app}/{name}/approve` | `action_approve` | Approve action |
| `POST` | `/api/v1/authenticated/admin/actions/{app}/{name}/reject` | `action_approve` | Reject action |
| `POST` | `/api/v1/authenticated/admin/actions/{app}/{name}/revoke` | `action_approve` | Revoke action |

#### Implementation Details

- **Data Model**: `scotty-core/src/settings/custom_action.rs`
- **API Handlers**: `scotty/src/api/rest/handlers/apps/custom_action*.rs`
- **CLI**: `scottyctl/src/commands/apps/actions.rs`, `scottyctl/src/commands/admin.rs`
- **Tests**: `scotty-core/src/settings/custom_action_tests.rs`

### scottyctl Architecture

**Command Structure**: Commands are organized hierarchically:
- `commands/`: Top-level command modules (apps, admin, auth)
  - `apps/`: App management commands (create, destroy, shell, logs, etc.)
  - `admin/`: Admin commands (scopes, roles, assignments)
  - `auth/`: OAuth login/logout

**Authentication**: Supports OAuth device flow and bearer tokens via environment variables or command-line args.

**File Upload (app:create)**:
- File collection happens in `scottyctl/src/utils/files.rs:collect_files()`
- Supports `.scottyignore` files using gitignore-style patterns (via the `ignore` crate)
- Files are base64-encoded and sent to the server
- Automatically excludes: `.DS_Store`, `.git/` directory
- `.scottyignore` patterns: `*.log`, `target`, `!important.log`, `**/*.tmp`, etc.

### Frontend-Backend Coupling

The Svelte frontend and Rust API are **tightly coupled** - breaking changes are acceptable. TypeScript types are generated from Rust using `ts-rs`. No API versioning or backwards compatibility needed.

## Configuration

### Server Configuration

Settings are loaded via the `config` crate:
1. Default values in code
2. Config files (YAML/JSON)
3. Environment variables (prefix: `SCOTTY__`)

Key server environment variables:
- `SCOTTY__API__AUTH_MODE`: Set to `dev` for local development (disables auth)
- `SCOTTY__TELEMETRY`: Enable metrics/traces (`metrics,traces`)
- `SCOTTY__API__BEARER_TOKENS__<NAME>`: Bearer token values (use env vars, not config files)

### scottyctl Configuration

scottyctl uses different environment variables:
- `SCOTTY_SERVER`: Server URL (default: `http://localhost:21342`)
- `SCOTTY_ACCESS_TOKEN`: Bearer token for authentication

## Testing

- Unit tests are colocated with implementation
- Integration tests in `scotty/tests/`
- Use `axum-test` for HTTP endpoint testing
- Use `wiremock` for mocking external services

## Observability

Run the observability stack (Grafana, Jaeger, VictoriaMetrics):
```bash
cd observability
docker-compose up -d
```

Access:
- Grafana: http://grafana.ddev.site (admin/admin)
- Jaeger: http://jaeger.ddev.site
- VictoriaMetrics: http://vm.ddev.site

Metrics include: log streaming, shell sessions, WebSocket connections, task execution, HTTP performance, memory usage.

## Release Process

Uses `cargo-release` with automatic changelog generation via `git-cliff`.

**Important**: Do not manually update changelogs. They are automatically generated during the release process from git history.

```bash
# Create new release (example)
cargo release --no-publish alpha -x --tag-prefix ""
```

The release process:
1. Runs `scripts/generate-changelogs.sh` with the new version (via pre-release-hook)
2. Generates changelogs for workspace root and all crates from git history
3. Updates version numbers in all Cargo.toml files
4. Creates and signs git tags
5. Commits all changes

Pre-push hook via `cargo-husky` runs automatically to enforce quality checks.

## Git Rules

- Never delete `frontend/build/.gitkeep` from git
- No emojis in commit messages
- Use conventional commits
