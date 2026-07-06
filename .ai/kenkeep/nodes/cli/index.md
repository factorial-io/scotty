# kenkeep Index: cli

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) to learn about: app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata. #file-transfer #cli #scottyctl
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) to learn about: app:purge keeps volumes/DBs, app:destroy removes everything including images. #cli #app-purge #app-destroy #gotcha
- Open [**app:shell security characteristics to account for**](practice-cli-app-shell-security-characteristics.md) to learn about: Shell sessions run as the container's default user, aren't logged by Scotty, and bypass app-level auth. #cli #app-shell #security
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) to learn about: Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends. #file-transfer #docker #streaming #http
- Open [**scottyctl: explicit access token wins over cached OAuth token**](practice-scottyctl-access-token-precedence.md) to learn about: In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token. #scottyctl #auth #oauth #cli

## Components (what exists)
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) to learn about: app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB). #cli #app-cp #permissions #authorization
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) to learn about: Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only. #cli #app-create #traefik #middleware
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) to learn about: Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set. #cli #app-create #traefik #seo
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) to learn about: CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping. #scottyctl #cli #docker
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) to learn about: Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore. #cli #scottyctl #map

## By topic

### #cli
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #scottyctl
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #app-create
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) — Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](../apps/logs/map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../apps/logs/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #file-transfer
- Open [**app:cp path-spec parsing and pipe-mode semantics**](practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.
### #traefik
- Open [**Start Traefik before local development**](../traefik/practice-local-dev-traefik-prereq.md) — Local dev requires Traefik running via docker compose in apps/traefik as a prerequisite.
- Open [**/apps is gitignored except the tracked apps/traefik local-dev setup**](../traefik/map-apps-is-gitignored-except-the-tracked-apps-traefik-local-dev-setup.md) — .gitignore excludes /apps/* but re-includes /apps/traefik (compose plus dynamic file-provider config) so the local-dev Traefik setup ships with the repo.
- Open [**Local *.localhost subdomains may not auto-resolve to 127.0.0.1**](../traefik/practice-localhost-subdomain-dns-gotcha.md) — Not all systems resolve wildcard *.localhost subdomains; add explicit /etc/hosts entries for each app hostname used in local dev.
### #app-cp
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) — app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
### #app-destroy
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) — app:purge keeps volumes/DBs, app:destroy removes everything including images.
### #app-purge
- Open [**app:purge only clears ephemeral data; app:destroy is the irreversible one**](practice-cli-app-purge-vs-destroy.md) — app:purge keeps volumes/DBs, app:destroy removes everything including images.
### #app-shell
- Open [**app:shell security characteristics to account for**](practice-cli-app-shell-security-characteristics.md) — Shell sessions run as the container's default user, aren't logged by Scotty, and bypass app-level auth.
### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../auth/authorization/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](../auth/bearer-tokens/practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](../auth/authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #gotcha
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../auth/oauth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #http
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.
### #map
- Open [**Blueprints are reusable app templates**](../apps/blueprints/map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #middleware
- Open [**app:create --registry and --middleware require server-side allow-listing**](map-cli-app-create-registry-middleware-allowlist.md) — Custom registries and middleware referenced by app:create must be pre-configured/allow-listed on the server; middleware is Traefik-only.
- Open [**Traefik middlewares: built-ins apply before custom, order matters**](../traefik/practice-traefik-middleware-ordering.md) — Basic-auth then robots built-ins always precede custom middlewares from .scotty.yml, applied in array order; names are case-sensitive.
### #oauth
- Open [**Public vs protected routes under OAuth mode**](../auth/oauth/map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](../auth/bearer-tokens/practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
- Open [**scottyctl: explicit access token wins over cached OAuth token**](practice-scottyctl-access-token-precedence.md) — In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
### #permissions
- Open [**app:cp permission split and transfer size limit**](map-cli-app-cp-permission-and-size-limit.md) — app:cp downloads need view permission, uploads need manage; transfers capped by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
- Open [**Root layout loads user permissions when the user is logged in**](../frontend/map-root-layout-loads-user-permissions-when-the-user-is-logged-in.md) — frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on isLoggedIn; permission-gated UI derives from permissionsLoaded.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../auth/authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #seo
- Open [**app:create injects a noindex X-Robots-Tag by default**](map-cli-app-create-robots-header-default.md) — Scotty adds X-Robots-Tag: none,noarchive,... to every app response unless --allow-robots is set.
### #streaming
- Open [**File transfers stream over HTTP chunked tar, not WebSocket**](practice-file-transfer-http-tar-streaming.md) — Container file transfer uses HTTP GET/PUT with application/x-tar bodies and bounded, streaming I/O on both ends.