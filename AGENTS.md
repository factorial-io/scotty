# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Scotty is a micro Platform-as-a-Service (PaaS) for managing Docker Compose-based applications:

- **scotty**: HTTP server (REST API + WebSocket) for managing Docker Compose apps
- **scottyctl**: CLI client for the scotty server
- **scotty-core**: Shared business logic (Docker operations, settings, tasks)
- **scotty-types**: Shared type definitions (TypeScript-compatible via ts-rs)
- **frontend**: SvelteKit web interface (tightly coupled with API, no backwards compatibility needed)
- **ts-generator**: Utility for generating TypeScript bindings from Rust types

## Development Commands

```bash
# Tests
cargo test                                              # Run all tests
cargo test test_name -- --nocapture                     # Specific test with output
RUST_LOG=debug cargo test test_name -p scotty -- --nocapture  # With debug logging

# Server (use .env file for SCOTTY__API__AUTH_MODE=dev etc.)
SCOTTY__API__AUTH_MODE=dev cargo run --bin scotty        # Dev mode (no auth)
RUST_LOG=info cargo run --bin scotty                     # With logging
cargo run --bin scotty -- config                         # View configuration

# scottyctl
cargo run --bin scottyctl -- <command>
cargo run --bin scottyctl -- --server http://localhost:21342 --access-token <token> app:list
# Or via env: SCOTTY_SERVER=http://localhost:21342 SCOTTY_ACCESS_TOKEN=<token>

# Frontend (uses bun, not npm)
cd frontend && bun install && bun run dev               # Development server
bun run build                                           # Production build
bun run check                                           # Type checking
bun run lint                                            # Prettier + ESLint (must pass before push)

# Prerequisites: start Traefik for local development
cd apps/traefik && docker compose up -d
```

## Architecture

### Scotty Server (`scotty/src/`)

**Entry Point**: `main.rs` — initializes AppState (settings, Docker client, task manager), sets up OpenTelemetry, spawns HTTP server and background tasks.

**Key Modules**:
- `api/router.rs`: Axum router with OpenAPI docs (utoipa)
- `api/rest/handlers/`: REST endpoints — `apps/` (create, list, run, actions, notifications), `admin/` (assignments, permissions, roles, scopes), `scopes/` (user-facing), `blueprints.rs`, `landing.rs` (Traefik fallback routing), `login.rs`, `tasks.rs`, `health.rs`, `info.rs`
- `api/websocket/`: Real-time features — `handlers/` (auth, logs, shell, tasks), `messaging.rs` (protocol), `client.rs` (connection mgmt)
- `api/auth_core.rs`: Core authentication logic
- `api/middleware/`: Casbin RBAC authorization
- `api/rate_limiting/`: Per-tier rate limiting
- `docker/state_machine_handlers/`: App lifecycle steps (create dir, save files, docker login, compose up, load balancer config, post actions, wait for containers, etc.)
- `docker/services/`: Long-running log streaming and shell sessions
- `docker/loadbalancer/`: Traefik/HAProxy config generation
- `onepassword/`: 1Password secrets — resolves `op://` URIs in app env vars (two-pass: 1Password lookup, then env var substitution)
- `oauth/`: OAuth 2.0 — device flow (CLI) and web flow (`/oauth/authorize`, `/api/oauth/callback`, `/oauth/exchange`)
- `services/authorization/`: Casbin RBAC (scopes, roles, permissions)
- `tasks/`: Task execution and output streaming
- `notification/`: Webhook, Mattermost, GitLab notifications
- `static_files.rs`: Embedded frontend serving
- `metrics/`: Collectors for log streaming, shell sessions, WebSocket connections, etc.

**AppState** (shared via `Arc`): Settings, Docker client (Bollard), task manager, authorization service, metrics collectors.

### Authorization System

Uses Casbin for RBAC. Config: `config/casbin/policy.yaml`. Implementation: `scotty/src/services/authorization/casbin.rs`. Tests: `scotty/tests/authorization_domain_test.rs`.

**Permissions**: `view`, `manage`, `create`, `destroy`, `shell`, `logs`, `admin_read`, `admin_write`

**Assignment matching** (by precedence): exact email (`user@factorial.io`) > domain pattern (`@factorial.io`) > wildcard (`*`). Wildcard is always additive. Domain patterns prevent subdomain attacks. Case-insensitive per RFC 5321.

```yaml
# config/casbin/policy.yaml
scopes:
  client-a: { description: "Client A Production" }
  qa: { description: "QA Environment" }
roles:
  admin: { permissions: ['*'], description: "Full access" }
  developer: { permissions: ['view', 'manage', 'create', 'shell', 'logs'], description: "Dev access" }
  viewer: { permissions: ['view'], description: "Read-only" }
assignments:
  stephan@factorial.io:                     # Exact match (highest priority)
    - { role: admin, scopes: ['*'] }
  '@factorial.io':                          # Domain match (fallback)
    - { role: developer, scopes: ['client-a', 'qa'] }
  '*':                                      # Wildcard (always additive)
    - { role: viewer, scopes: ['default'] }
```

### scottyctl (`scottyctl/src/`)

**Commands** (colon-separated namespace):
- `app:` list, create, destroy, run, start, stop, rebuild, purge, adopt, info, action, logs, shell
- `admin:` scopes:\*, roles:\*, assignments:\*, permissions:\*
- `auth:` login, logout, status, refresh
- `blueprint:` list, info
- `notify:` add, remove
- `completion`, `test`

**Global flags**: `--server`, `--access-token`, `--debug`, `--bypass-version-check`

**Preflight** (`preflight.rs`): Checks client/server version compatibility via `/api/v1/info` before running commands. Bypass with `--bypass-version-check`.

**File upload** (`app:create`): Files collected via `utils/files.rs:collect_files()`, base64-encoded. Supports `.scottyignore` (gitignore-style patterns via `ignore` crate). Auto-excludes `.DS_Store`, `.git/`.

**Auth**: OAuth device flow + bearer tokens via env vars or CLI args. Core logic in `auth/` (device flow, token storage, caching).

### Blueprints

Reusable app templates defining required/public services, port mappings, lifecycle actions (PostCreate, PostRun, PostRebuild), and custom actions per service. Available via `blueprint:list`/`blueprint:info` and `GET /api/v1/authenticated/blueprints`.

## Configuration

Settings loaded via `config` crate: 1) defaults in code, 2) config files (YAML/TOML), 3) env vars (prefix: `SCOTTY__`).

**Server env vars**: `SCOTTY__API__AUTH_MODE=dev` (disable auth), `SCOTTY__TELEMETRY=metrics,traces`, `SCOTTY__API__BEARER_TOKENS__<NAME>` (use env vars, not config files).

**scottyctl env vars**: `SCOTTY_SERVER` (default: `http://localhost:21342`), `SCOTTY_ACCESS_TOKEN`.

## Testing

Unit tests colocated with implementation. Integration tests in `scotty/tests/`. Uses `axum-test` for HTTP testing, `wiremock` for mocking external services.

## Observability

```bash
cd observability && docker compose up -d
```
Grafana: http://grafana.ddev.site (admin/admin) | Jaeger: http://jaeger.ddev.site | VictoriaMetrics: http://vm.ddev.site

## Release Process

Uses `cargo-release` with `git-cliff` for automatic changelog generation. Do not manually update changelogs.

```bash
cargo release --no-publish alpha -x --tag-prefix ""
```

Runs `scripts/generate-changelogs.sh`, updates versions in all Cargo.toml files, creates signed git tags. Pre-push hook via `cargo-husky` enforces quality checks.

## Project Management

Uses **beans**, an agentic-first issue tracker. Issues ("beans") are managed via the `beans` CLI. The `.beans/` directory is committed to the repository. Agents should use beans instead of todo lists to track work, create/update issues, and manage task dependencies.

## Git Rules

- Never delete `frontend/build/.gitkeep` from git
- No emojis in commit messages
- Use conventional commits
