# kenkeep Index: frontend

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) to learn about: Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable. #frontend #api #architecture
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) to learn about: Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push. #frontend #tooling #bun
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) to learn about: After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/. #frontend #types #ts-rs #workflow

## Components (what exists)
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) to learn about: Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend. #frontend #structure #sveltekit

## By topic

### #frontend
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../apps/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](../apps/practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #api
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Rate limiting has three independent tiers keyed differently**](../auth/map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](../observability/practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #bun
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #structure
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
- Open [**Which files under config/ are committed vs git-ignored**](../configuration/map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #sveltekit
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
### #tooling
- Open [**Pre-push git hook installed via cargo-husky**](../workflow/map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #ts-rs
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #types
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #workflow
- Open [**Custom actions require approval before execution**](../apps/map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](../workflow/practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](../workflow/practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.