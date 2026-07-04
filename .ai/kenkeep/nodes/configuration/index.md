# kenkeep Index: configuration

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](practice-access-token-config-removed-use-bearer-tokens.md) to learn about: api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens. #auth #configuration #gotcha
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) to learn about: Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored. #configuration #env #local-development
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) to learn about: If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps. #docker #configuration #gotcha
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) to learn about: Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores. #configuration #env
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) to learn about: Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry. #observability #config #env-vars
- Open [**Never store real bearer tokens in configuration files**](practice-never-store-real-bearer-tokens-in-config.md) to learn about: Keep only placeholder values for api.bearer_tokens in config files; supply real secrets via SCOTTY__API__BEARER_TOKENS__<NAME> env vars. #security #auth #configuration #secrets
- Open [**Settings load order and secret handling**](practice-settings-config-precedence.md) to learn about: Config precedence: code defaults, then config files, then SCOTTY__-prefixed env vars; use env vars (not config files) for bearer tokens. #configuration #settings #security

## Components (what exists)
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) to learn about: Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml. #1password #secrets #configuration
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) to learn about: The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime. #docker #deployment #config #security
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) to learn about: config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets). #config #casbin #git #structure

## By topic

### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #config
- Open [**Apps declare authorization scopes in .scotty.yml**](../apps/map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../auth/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../auth/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](../auth/practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](../apps/map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../apps/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #env
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](../apps/map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
### #gotcha
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../auth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #secrets
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) — Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml.
- Open [**Never store real bearer tokens in configuration files**](practice-never-store-real-bearer-tokens-in-config.md) — Keep only placeholder values for api.bearer_tokens in config files; supply real secrets via SCOTTY__API__BEARER_TOKENS__<NAME> env vars.
### #1password
- Open [**1Password secrets are resolved via op:// URIs in app env vars only**](map-onepassword-secret-resolution.md) — Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in env vars passed via app:create, not inside compose.yml.
### #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../auth/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #deployment
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](../auth/practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #env-vars
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
### #git
- Open [**Git conventions for this repo**](../workflow/practice-git-rules.md) — Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits.
- Open [**Pre-push git hook installed via cargo-husky**](../workflow/map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #local-development
- Open [**.env / .env.local precedence and usage convention**](practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
### #observability
- Open [**Scotty metrics families and prefix**](../observability/map-scotty-metrics-families.md) — All Scotty metrics use the scotty_ prefix, grouped by subsystem: log streaming, shell sessions, websocket, tasks, HTTP server, memory, application fleet, and Tokio runtime.
- Open [**Observability stack config file locations**](../observability/map-observability-config-files.md) — docker-compose.yml, otel-collector-config.yaml, and grafana/ provisioning/dashboards dirs define the observability stack's setup.
- Open [**Observability stack architecture**](../observability/map-observability-stack-architecture.md) — Scotty exports OTLP telemetry to an OpenTelemetry Collector, which routes traces to Jaeger and metrics to VictoriaMetrics, visualized in Grafana.
### #settings
- Open [**Settings load order and secret handling**](practice-settings-config-precedence.md) — Config precedence: code defaults, then config files, then SCOTTY__-prefixed env vars; use env vars (not config files) for bearer tokens.
### #structure
- Open [**Frontend src/ layout and dev-server proxy targets**](../frontend/map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
- Open [**Which files under config/ are committed vs git-ignored**](map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).