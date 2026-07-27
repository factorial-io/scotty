---
schema_version: 1
summaries:
  apps: >-
    app lifecycle, anatomy, blueprints, custom actions, and container status/log
    behavior; read when creating or managing apps, changing lifecycle state
    machines, or touching log streaming
  apps/anatomy: >-
    what makes a folder an app, .scotty.yml scope declaration, and which files
    Scotty may write; read when creating apps or touching files in an app
    directory
  apps/blueprints: >-
    blueprint templates and the env vars injected into blueprint action scripts;
    read when authoring or changing blueprints
  apps/custom-actions: >-
    custom action approval workflow and execution gating; read when changing
    custom actions or their permissions
  apps/lifecycle: >-
    app types (owned/supported/unsupported), status aggregation, and
    state-machine failure behavior; read when changing app lifecycle, status
    logic, or adopting unsupported apps
  apps/logs: >-
    container log streaming behavior for stopped and missing containers,
    including follow mode; read when touching log streaming or the logs
    websocket
  architecture: >-
    workspace crates, server module map, product scope, and cross-cutting
    runtime invariants; read when orienting in the codebase or adding a module
  auth: >-
    authentication (OAuth, bearer tokens), Casbin RBAC authorization, and rate
    limiting; read when touching auth flows, permissions, policy.yaml, or rate
    limits
  auth/authorization: >-
    Casbin RBAC: built-in roles, assignment key formats per auth mode,
    precedence rules, and required token assignments; read when changing
    policy.yaml, roles, scopes, or permissions
  auth/bearer-tokens: >-
    bearer token naming, constant-time comparison, and hybrid-mode check order;
    read when touching bearer token auth or service-account access
  auth/oauth: >-
    OAuth route protection, secret storage, session lifetimes, and the
    redirect_url vs frontend_base_url distinction; read when changing OAuth
    flows or their configuration
  auth/rate-limiting: >-
    per-tier rate limiting and its per-instance (not global) scope; read when
    changing rate limits or scaling the server
  cli: >-
    scottyctl command structure, app:cp file transfer, and CLI-facing behavior;
    read when changing scottyctl commands or the endpoints they call
  configuration: >-
    settings precedence, SCOTTY__ env var conventions, secrets handling, and
    config file layout; read when adding settings, handling secrets, or
    deploying the server
  frontend: >-
    SvelteKit frontend layout, bun tooling, and ts-rs type generation; read when
    changing the frontend or Rust types shared with it
  observability: >-
    the OTLP collector, Grafana/Jaeger/VictoriaMetrics stack, and scotty_ metric
    families; read when adding metrics, traces, or dashboards
  traefik: >-
    Traefik load balancer integration, per-app proxy networks, middleware rules,
    and the default-backend landing page; read when changing routing, networks,
    or local-dev proxy setup
  workflow: >-
    VCS (jj) usage, git rules, release automation, beans issue tracking, testing
    and contribution conventions; read before committing, releasing, tracking
    work, or onboarding
---
# kenkeep Folder Summaries

- `apps`: app lifecycle, anatomy, blueprints, custom actions, and container status/log behavior; read when creating or managing apps, changing lifecycle state machines, or touching log streaming
- `apps/anatomy`: what makes a folder an app, .scotty.yml scope declaration, and which files Scotty may write; read when creating apps or touching files in an app directory
- `apps/blueprints`: blueprint templates and the env vars injected into blueprint action scripts; read when authoring or changing blueprints
- `apps/custom-actions`: custom action approval workflow and execution gating; read when changing custom actions or their permissions
- `apps/lifecycle`: app types (owned/supported/unsupported), status aggregation, and state-machine failure behavior; read when changing app lifecycle, status logic, or adopting unsupported apps
- `apps/logs`: container log streaming behavior for stopped and missing containers, including follow mode; read when touching log streaming or the logs websocket
- `architecture`: workspace crates, server module map, product scope, and cross-cutting runtime invariants; read when orienting in the codebase or adding a module
- `auth`: authentication (OAuth, bearer tokens), Casbin RBAC authorization, and rate limiting; read when touching auth flows, permissions, policy.yaml, or rate limits
- `auth/authorization`: Casbin RBAC: built-in roles, assignment key formats per auth mode, precedence rules, and required token assignments; read when changing policy.yaml, roles, scopes, or permissions
- `auth/bearer-tokens`: bearer token naming, constant-time comparison, and hybrid-mode check order; read when touching bearer token auth or service-account access
- `auth/oauth`: OAuth route protection, secret storage, session lifetimes, and the redirect_url vs frontend_base_url distinction; read when changing OAuth flows or their configuration
- `auth/rate-limiting`: per-tier rate limiting and its per-instance (not global) scope; read when changing rate limits or scaling the server
- `cli`: scottyctl command structure, app:cp file transfer, and CLI-facing behavior; read when changing scottyctl commands or the endpoints they call
- `configuration`: settings precedence, SCOTTY__ env var conventions, secrets handling, and config file layout; read when adding settings, handling secrets, or deploying the server
- `frontend`: SvelteKit frontend layout, bun tooling, and ts-rs type generation; read when changing the frontend or Rust types shared with it
- `observability`: the OTLP collector, Grafana/Jaeger/VictoriaMetrics stack, and scotty_ metric families; read when adding metrics, traces, or dashboards
- `traefik`: Traefik load balancer integration, per-app proxy networks, middleware rules, and the default-backend landing page; read when changing routing, networks, or local-dev proxy setup
- `workflow`: VCS (jj) usage, git rules, release automation, beans issue tracking, testing and contribution conventions; read before committing, releasing, tracking work, or onboarding
