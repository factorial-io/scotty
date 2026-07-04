# kenkeep Index: traefik

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Default-backend setup requires api.base_url and a low-priority catch-all router**](practice-default-backend-configuration.md) to learn about: Landing-page redirects only work if api.base_url is set and a lowest-priority catch-all Traefik router forwards unmatched domains to Scotty. #traefik #configuration #landing-page #gotcha
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) to learn about: Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev. #local-dev #dns #traefik #gotcha
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) to learn about: Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them. #traefik #docker #networking #migration
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) to learn about: Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite. #local-dev #traefik #prerequisites
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) to learn about: Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive. #traefik #middleware #scotty-yml #loadbalancer

## Components (what exists)
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) to learn about: The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app. #security #landing-page #authorization
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) to learn about: Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions. #traefik #docker #networking #loadbalancer
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) to learn about: Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error. #landing-page #traefik #load-balancer #architecture
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) to learn about: Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support. #scotty #traefik #haproxy #load-balancer

## By topic

### #traefik
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**app:create --registry and --middleware require server-side allow-listing**](../cli/map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #landing-page
- Open [**Default-backend setup requires api.base_url and a low-priority catch-all router**](practice-default-backend-configuration.md) — Landing-page redirects only work if api.base_url is set and a lowest-priority catch-all Traefik router forwards unmatched domains to Scotty.
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) — Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error.
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](../apps/map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../apps/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #gotcha
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../auth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #load-balancer
- Open [**Scotty as Traefik default backend / landing page**](map-default-backend-landing-page.md) — Scotty can serve as the load balancer's catch-all backend, showing a Start-app landing page for stopped apps instead of a gateway error.
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) — Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support.
### #loadbalancer
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #local-dev
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
- Open [**Local observability stack: prerequisite and access URLs**](../observability/map-observability-local-access.md) — The observability stack (observability/docker-compose) needs Traefik running first for .ddev.site routing; Grafana/Jaeger/VictoriaMetrics are reached via *.ddev.site URLs.
### #networking
- Open [**Each app gets its own dedicated Traefik proxy network**](map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) — Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them.
### #architecture
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](../frontend/practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Observability backends are swappable via open standards**](../observability/practice-observability-stack-swappable.md) — Scotty's telemetry uses OTLP, PromQL, and W3C Trace Context so any component (VictoriaMetrics, Jaeger, Grafana) can be replaced with a compatible alternative.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](../auth/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #dns
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
### #haproxy
- Open [**Scotty supports Traefik and legacy Haproxy-config load balancers**](map-load-balancer-support.md) — Traefik is the primary supported load balancer; haproxy-config is legacy/deprecated and lacks robots-blocking support.
### #middleware
- Open [**app:create --registry and --middleware require server-side allow-listing**](../cli/map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #migration
- Open [**Migrating an app from the old shared Traefik network requires app:rebuild**](practice-traefik-network-migration-requires-rebuild.md) — Apps created before the per-app-network change keep using the old shared network until you run app:rebuild; app:run does not migrate them.
### #prerequisites
- Open [**Start Traefik before local development**](practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
### #scotty
- Open [**An app is any folder in the apps directory containing compose.yml**](../apps/map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](../apps/map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
- Open [**compose.yml must not expose ports directly or use env-var expansion**](../apps/practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #scotty-yml
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../auth/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.