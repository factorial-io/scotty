---
okf_version: '0.1'
---
# kenkeep Index

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
- Load [`apps/`](apps/index.md) for more information on app lifecycle, anatomy, blueprints, custom actions, and container status/log behavior; read when creating or managing apps, changing lifecycle state machines, or touching log streaming.
- Load [`architecture/`](architecture/index.md) for more information on workspace crates, server module map, product scope, and cross-cutting runtime invariants; read when orienting in the codebase or adding a module.
- Load [`auth/`](auth/index.md) for more information on authentication (OAuth, bearer tokens), Casbin RBAC authorization, and rate limiting; read when touching auth flows, permissions, policy.yaml, or rate limits.
- Load [`cli/`](cli/index.md) for more information on scottyctl command structure, app:cp file transfer, and CLI-facing behavior; read when changing scottyctl commands or the endpoints they call.
- Load [`configuration/`](configuration/index.md) for more information on settings precedence, SCOTTY__ env var conventions, secrets handling, and config file layout; read when adding settings, handling secrets, or deploying the server.
- Load [`frontend/`](frontend/index.md) for more information on SvelteKit frontend layout, bun tooling, and ts-rs type generation; read when changing the frontend or Rust types shared with it.
- Load [`observability/`](observability/index.md) for more information on the OTLP collector, Grafana/Jaeger/VictoriaMetrics stack, and scotty_ metric families; read when adding metrics, traces, or dashboards.
- Load [`traefik/`](traefik/index.md) for more information on Traefik load balancer integration, per-app proxy networks, middleware rules, and the default-backend landing page; read when changing routing, networks, or local-dev proxy setup.
- Load [`workflow/`](workflow/index.md) for more information on VCS (jj) usage, git rules, release automation, beans issue tracking, testing and contribution conventions; read before committing, releasing, tracking work, or onboarding.

## Conventions (how we build)
_None yet._

## Components (what exists)
_None yet._

## By topic

_No tags yet._
