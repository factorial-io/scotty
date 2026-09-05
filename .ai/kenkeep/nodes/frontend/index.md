# kenkeep Index: frontend

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) to learn about: Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable. #frontend #api #architecture
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) to learn about: Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push. #frontend #tooling #bun
- Open [**Frontend unit tests run with Vitest, colocated as *.test.ts**](practice-frontend-unit-tests-vitest.md) to learn about: \`bun run test\` runs Vitest through vite.config.ts (jsdom, src/**/*.test.ts); tests sit next to the module, mock $app/* and stores with vi.mock, stub browser APIs with vi.stubGlobal, and set the page URL via a @vitest-environment-options docblock. No component tests yet. #frontend #testing #vitest #workflow
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) to learn about: After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/. #frontend #types #ts-rs #workflow

## Components (what exists)
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) to learn about: Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend. #frontend #structure #sveltekit
- Open [**Root layout loads user permissions when the user is logged in**](map-root-layout-loads-user-permissions-when-the-user-is-logged-in.md) to learn about: frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on isLoggedIn; permission-gated UI derives from permissionsLoaded. #frontend #permissions #svelte

## By topic

### #frontend
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
### #workflow
- Open [**Frontend unit tests run with Vitest, colocated as *.test.ts**](practice-frontend-unit-tests-vitest.md) — \`bun run test\` runs Vitest through vite.config.ts (jsdom, src/**/*.test.ts); tests sit next to the module, mock $app/* and stores with vi.mock, stub browser APIs with vi.stubGlobal, and set the page URL via a @vitest-environment-options docblock. No component tests yet.
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
- Open [**Custom actions require approval before execution**](../apps/custom-actions/map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
### #api
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Rate limiting has three independent tiers keyed differently**](../auth/rate-limiting/map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
- Open [**App-create file content is a base64 string on the wire**](../apps/lifecycle/map-app-create-file-content-is-a-base64-string-on-the-wire.md) — File.content serializes as a base64 string; deserialization also accepts a legacy JSON int array and strips the extra base64 layer.
### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](../observability/practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #bun
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #permissions
- Open [**app:cp permission split and transfer size limit**](../cli/map-cli-app-cp-permission-and-size-limit.md) — app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
- Open [**Root layout loads user permissions when the user is logged in**](map-root-layout-loads-user-permissions-when-the-user-is-logged-in.md) — frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on isLoggedIn; permission-gated UI derives from permissionsLoaded.
### #structure
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
- Open [**Which files under config/ are committed vs git-ignored**](../configuration/map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #svelte
- Open [**Root layout loads user permissions when the user is logged in**](map-root-layout-loads-user-permissions-when-the-user-is-logged-in.md) — frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on isLoggedIn; permission-gated UI derives from permissionsLoaded.
### #sveltekit
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
### #testing
- Open [**Frontend unit tests run with Vitest, colocated as *.test.ts**](practice-frontend-unit-tests-vitest.md) — \`bun run test\` runs Vitest through vite.config.ts (jsdom, src/**/*.test.ts); tests sit next to the module, mock $app/* and stores with vi.mock, stub browser APIs with vi.stubGlobal, and set the page URL via a @vitest-environment-options docblock. No component tests yet.
- Open [**Test placement and tooling conventions**](../workflow/practice-testing-conventions.md) — Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking.
### #tooling
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
- Open [**Pre-push git hook installed via cargo-husky**](../workflow/map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #ts-rs
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #types
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #vitest
- Open [**Frontend unit tests run with Vitest, colocated as *.test.ts**](practice-frontend-unit-tests-vitest.md) — \`bun run test\` runs Vitest through vite.config.ts (jsdom, src/**/*.test.ts); tests sit next to the module, mock $app/* and stores with vi.mock, stub browser APIs with vi.stubGlobal, and set the page URL via a @vitest-environment-options docblock. No component tests yet.