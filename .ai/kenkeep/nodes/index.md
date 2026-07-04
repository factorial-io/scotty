---
okf_version: '0.1'
---
# kenkeep Index

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) to learn about: Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored. #configuration #env #local-development
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](practice-access-token-config-removed-use-bearer-tokens.md) to learn about: The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead. #auth #configuration #gotcha
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) to learn about: app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata. #file-transfer #cli #scottyctl
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) to learn about: app:purge keeps volumes/DBs, app:destroy removes everything including images. #cli #app-purge #app-destroy #gotcha
- Open [**app:shell security characteristics to account for**](practice-cli-app-shell-security-characteristics.md) to learn about: Shell sessions run as the container's default user, aren't logged by Scotty, and bypass app-level auth. #cli #app-shell #security
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) to learn about: If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps. #docker #configuration #gotcha
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) to learn about: Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison. #security #auth #bearer-token
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) to learn about: policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email. #auth #casbin #bearer-tokens #naming
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) to learn about: Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token. #authorization #bearer-token #casbin #auth
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) to learn about: Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive. #authorization #casbin #security
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) to learn about: Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion. #scotty #compose #restriction
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) to learn about: Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores. #configuration #env
- Open [**Default-backend setup requires api.base_url and a low-priority catch-all router**](practice-default-backend-configuration.md) to learn about: Landing-page redirects only work if api.base_url is set and a lowest-priority catch-all Traefik router forwards unmatched domains to Scotty. #traefik #configuration #landing-page #gotcha
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) to learn about: Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry. #observability #config #env-vars
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) to learn about: Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends. #file-transfer #docker #streaming #http
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) to learn about: Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable. #frontend #api #architecture
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) to learn about: Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push. #frontend #tooling #bun
- Open [**Git conventions for this repo**](practice-git-rules.md) to learn about: Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits. #git #conventions
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) to learn about: Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency. #oauth #bearer-tokens #auth #performance
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) to learn about: Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev. #local-dev #dns #traefik #gotcha
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) to learn about: Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them. #traefik #docker #networking #migration
- Open [**Never store real bearer tokens in configuration files**](practice-never-store-real-bearer-tokens-in-config.md) to learn about: Keep only placeholder values for api.bearer_tokens in config files; supply real secrets via SCOTTY__API__BEARER_TOKENS__<NAME> env vars. #security #auth #configuration #secrets
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) to learn about: redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to. #oauth #configuration #gotcha
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) to learn about: Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative. #observability #prometheus #architecture
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) to learn about: In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N. #rate-limiting #deployment #gotcha
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) to learn about: After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/. #frontend #types #ts-rs #workflow
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) to learn about: Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release. #release #versioning #ci
- Open [**Run app:rebuild after app:adopt to actually enable load balancing**](practice-adopt-then-rebuild-for-unsupported-apps.md) to learn about: app:adopt only writes Scotty settings; it does not apply load balancer config — app:rebuild is required afterward. #scotty #cli #workflow #gotcha
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) to learn about: App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status. #docker #status #container #frontend
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) to learn about: Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml. #scotty #compose #file-ownership
- Open [**scottyctl: explicit access token wins over cached OAuth token**](practice-scottyctl-access-token-precedence.md) to learn about: In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token. #scottyctl #auth #oauth #cli
- Open [**Settings load order and secret handling**](practice-settings-config-precedence.md) to learn about: Config precedence: code defaults, then config files, then SCOTTY__-prefixed env vars; use env vars (not config files) for bearer tokens. #configuration #settings #security
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) to learn about: Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite. #local-dev #traefik #prerequisites
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) to learn about: Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking. #testing #conventions
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) to learn about: Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive. #traefik #middleware #scotty-yml #loadbalancer
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) to learn about: Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans. #project-management #beans #workflow

## Components (what exists)
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) to learn about: Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml. #1password #secrets #configuration
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) to learn about: Apps are identified by folder name; service hostnames derive from app name + service name unless overridden. #scotty #apps #vocabulary
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) to learn about: app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB). #cli #app-cp #permissions #authorization
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) to learn about: Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only. #cli #app-create #traefik #middleware
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) to learn about: Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set. #cli #app-create #traefik #seo
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) to learn about: Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees. #scotty #apps #vocabulary
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) to learn about: App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`. #authorization #scopes #config
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) to learn about: RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs. #authorization #casbin #security #map
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) to learn about: Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands. #blueprints #configuration #env
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) to learn about: Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions. #blueprints #map
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) to learn about: Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver. #authorization #rbac #roles #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) to learn about: OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments. #authorization #casbin #auth
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) to learn about: run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes. #custom-actions #authorization #blueprints
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) to learn about: Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions. #custom-actions #workflow #security
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) to learn about: The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app. #security #landing-page #authorization
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) to learn about: Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions. #traefik #docker #networking #loadbalancer
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) to learn about: Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError. #logs #docker #websocket #frontend
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) to learn about: Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend. #frontend #structure #sveltekit
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) to learn about: The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs. #observability #ddev #traefik #local-dev
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) to learn about: Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error. #logs #docker #websocket
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) to learn about: PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes. #oauth #security #sessions
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) to learn about: Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana. #observability #opentelemetry #metrics #tracing #grafana
- Open [**Observability stack config file locations**](map-observability-config-files.md) to learn about: docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup. #observability #grafana #configuration
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) to learn about: VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart. #observability #jaeger #victoriametrics
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) to learn about: The project uses a pre-push git hook installed by cargo-husky, set up automatically. #git #tooling #hooks #husky
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) to learn about: Scotty's route protection split: which paths are public and which require authentication. #oauth #auth #routing #security
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) to learn about: public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user). #rate-limiting #security #api
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) to learn about: scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked. #tls #rustls #startup #docker-build
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) to learn about: Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error. #landing-page #traefik #load-balancer #architecture
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) to learn about: The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime. #docker #deployment #config #security
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) to learn about: All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime. #observability #metrics
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) to learn about: Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities. #architecture #server #map
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) to learn about: Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support. #scotty #traefik #haproxy #load-balancer
- Open [**Scotty workspace components**](map-project-workspace-components.md) to learn about: The crates/apps making up the Scotty micro-PaaS and what each one does. #architecture #workspace #overview
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) to learn about: Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement. #architecture #scope #positioning
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) to learn about: CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping. #scottyctl #cli #docker
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) to learn about: Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore. #cli #scottyctl #map
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) to learn about: App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever. #state-machine #tasks #docker
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) to learn about: config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets). #config #casbin #git #structure

## By topic

### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #traefik
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #cli
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #gotcha
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
### #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #observability
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) — All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime.
- Open [**Observability stack config file locations**](map-observability-config-files.md) — docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup.
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #frontend
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #scotty
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #oauth
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
- Open [**scottyctl: explicit access token wins over cached OAuth token**](practice-scottyctl-access-token-precedence.md) — In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
### #config
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #map
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #scottyctl
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #workflow
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #blueprints
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
### #env
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
### #git
- Open [**Git conventions for this repo**](practice-git-rules.md) — Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits.
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #landing-page
- Open [**Default-backend setup requires api.base_url and a low-priority catch-all router**](practice-default-backend-configuration.md) — Landing-page redirects only work if api.base_url is set and a lowest-priority catch-all Traefik router forwards unmatched domains to Scotty.
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) — Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error.
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
### #local-dev
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) — The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs.
### #api
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
### #app-create
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) — Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set.
### #apps
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #bearer-token
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #bearer-tokens
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #compose
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #conventions
- Open [**Git conventions for this repo**](practice-git-rules.md) — Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits.
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) — Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking.
### #custom-actions
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
### #deployment
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #file-transfer
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.
### #grafana
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
- Open [**Observability stack config file locations**](map-observability-config-files.md) — docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup.
### #load-balancer
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) — Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error.
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) — Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support.
### #loadbalancer
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #logs
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #metrics
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
- Open [**Scotty metrics families and prefix**](map-scotty-metrics-families.md) — All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime.
### #middleware
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #networking
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) — Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them.
### #rate-limiting
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
### #secrets
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) — Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml.
- Open [**Never store real bearer tokens in configuration files**](practice-never-store-real-bearer-tokens-in-config.md) — Keep only placeholder values for api.bearer_tokens in config files; supply real secrets via SCOTTY__API__BEARER_TOKENS__<NAME> env vars.
### #structure
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #tooling
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #vocabulary
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #websocket
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #1password
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) — Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml.
### #app-cp
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) — app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
### #app-destroy
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) — app:purge keeps volumes/DBs, app:destroy removes everything including images.
### #app-purge
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) — app:purge keeps volumes/DBs, app:destroy removes everything including images.
### #app-shell
- Open [**app:shell security characteristics to account for**](practice-cli-app-shell-security-characteristics.md) — Shell sessions run as the container's default user, aren't logged by Scotty, and bypass app-level auth.
### #beans
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
### #bun
- Open [**Frontend tooling uses bun, not npm**](practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #ci
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.
### #container
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #ddev
- Open [**Local observability stack: prerequisite and access URLs**](map-observability-local-access.md) — The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs.
### #dns
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
### #docker-build
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #env-vars
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
### #file-ownership
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #haproxy
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) — Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support.
### #hooks
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #http
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.
### #husky
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #jaeger
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) — VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart.
### #local-development
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
### #migration
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) — Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them.
### #naming
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
### #opentelemetry
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #overview
- Open [**Scotty workspace components**](map-project-workspace-components.md) — The crates/apps making up the Scotty micro-PaaS and what each one does.
### #performance
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #permissions
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) — app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
### #positioning
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) — Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement.
### #prerequisites
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
### #project-management
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
### #prometheus
- Open [**Observability backends are swappable via open standards**](practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
### #rbac
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.
### #release
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.
### #restriction
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #roles
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.
### #routing
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
### #rustls
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #scope
- Open [**Scotty's scope: single-node micro-PaaS, not a cluster orchestrator**](map-scotty-product-scope.md) — Scotty is a single-node docker-compose orchestrator for ephemeral review apps, not a Kubernetes/Nomad replacement.
### #scopes
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
### #scotty-yml
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #seo
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) — Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set.
### #server
- Open [**Scotty server key modules and their locations**](map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #sessions
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) — PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes.
### #settings
- Open [**Settings load order and secret handling**](practice-settings-config-precedence.md) — Config precedence: code defaults, then config files, then SCOTTY__-prefixed env vars; use env vars (not config files) for bearer tokens.
### #startup
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #state-machine
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.
### #status
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #streaming
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.
### #sveltekit
- Open [**Frontend src/ layout and dev-server proxy targets**](map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
### #tasks
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.
### #testing
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) — Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking.
### #tls
- Open [**rustls CryptoProvider is installed explicitly at process start**](map-rustls-crypto-provider-init.md) — scotty and scottyctl both call scotty_core::http::ensure_crypto_provider() before any TLS use, and Docker builds pin dependency versions with --locked.
### #tracing
- Open [**Observability stack architecture**](map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #ts-rs
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #types
- Open [**Regenerate frontend TypeScript types after backend Rust type changes**](practice-frontend-types-regenerate-after-backend-change.md) — After changing Rust types, run \`cargo run --bin ts-generator\` from the repo root to refresh generated TypeScript in frontend/src/generated/.
### #versioning
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.
### #victoriametrics
- Open [**Observability stack data retention limits**](map-observability-data-retention.md) — VictoriaMetrics retains metrics 30 days by default (configurable); Jaeger traces are in-memory only and lost on restart.
### #workspace
- Open [**Scotty workspace components**](map-project-workspace-components.md) — The crates/apps making up the Scotty micro-PaaS and what each one does.
