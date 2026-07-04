---
schema_version: 1
summaries:
  apps: >-
    app lifecycle, anatomy, blueprints, custom actions, and container status/log
    behavior; read when creating or managing apps, changing lifecycle state
    machines, or touching log streaming
  architecture: >-
    workspace crates, server module map, product scope, and cross-cutting
    runtime invariants; read when orienting in the codebase or adding a module
  auth: >-
    authentication (OAuth, bearer tokens), Casbin RBAC authorization, and rate
    limiting; read when touching auth flows, permissions, policy.yaml, or rate
    limits
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
- `architecture`: workspace crates, server module map, product scope, and cross-cutting runtime invariants; read when orienting in the codebase or adding a module
- `auth`: authentication (OAuth, bearer tokens), Casbin RBAC authorization, and rate limiting; read when touching auth flows, permissions, policy.yaml, or rate limits
- `cli`: scottyctl command structure, app:cp file transfer, and CLI-facing behavior; read when changing scottyctl commands or the endpoints they call
- `configuration`: settings precedence, SCOTTY__ env var conventions, secrets handling, and config file layout; read when adding settings, handling secrets, or deploying the server
- `frontend`: SvelteKit frontend layout, bun tooling, and ts-rs type generation; read when changing the frontend or Rust types shared with it
- `observability`: the OTLP collector, Grafana/Jaeger/VictoriaMetrics stack, and scotty_ metric families; read when adding metrics, traces, or dashboards
- `traefik`: Traefik load balancer integration, per-app proxy networks, middleware rules, and the default-backend landing page; read when changing routing, networks, or local-dev proxy setup
- `workflow`: VCS (jj) usage, git rules, release automation, beans issue tracking, testing and contribution conventions; read before committing, releasing, tracking work, or onboarding
