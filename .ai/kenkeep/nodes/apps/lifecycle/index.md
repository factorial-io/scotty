# kenkeep Index: apps / lifecycle

↑ Parent: [apps](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) to learn about: Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion. #scotty #compose #restriction
- Open [**Run app:rebuild after app:adopt to actually enable load balancing**](practice-adopt-then-rebuild-for-unsupported-apps.md) to learn about: app:adopt only writes Scotty settings; it does not apply load balancer config — app:rebuild is required afterward. #scotty #cli #workflow #gotcha
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) to learn about: App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status. #docker #status #container #frontend

## Components (what exists)
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) to learn about: Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees. #scotty #apps #vocabulary
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) to learn about: App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever. #state-machine #tasks #docker

## By topic

### #scotty
- Open [**An app is any folder in the apps directory containing compose.yml**](../anatomy/map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](../logs/map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../logs/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](../../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #apps
- Open [**An app is any folder in the apps directory containing compose.yml**](../anatomy/map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #cli
- Open [**app:cp path-spec parsing and pipe-mode semantics**](../../cli/practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](../../cli/map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](../../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #compose
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](../anatomy/practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #container
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #frontend
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](../logs/map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
- Open [**Frontend src/ layout and dev-server proxy targets**](../../frontend/map-frontend-src-layout.md) — Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts), generated (ts-rs output), and lib; dev server proxies /api and /ws to the backend.
### #gotcha
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../../auth/oauth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #restriction
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #state-machine
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.
### #status
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #tasks
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.
### #vocabulary
- Open [**An app is any folder in the apps directory containing compose.yml**](../anatomy/map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #workflow
- Open [**Custom actions require approval before execution**](../custom-actions/map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](../../workflow/practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](../../workflow/practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.