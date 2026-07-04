# kenkeep Index: apps

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) to learn about: Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion. #scotty #compose #restriction
- Open [**Run app:rebuild after app:adopt to actually enable load balancing**](practice-adopt-then-rebuild-for-unsupported-apps.md) to learn about: app:adopt only writes Scotty settings; it does not apply load balancer config — app:rebuild is required afterward. #scotty #cli #workflow #gotcha
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) to learn about: App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status. #docker #status #container #frontend
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) to learn about: Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml. #scotty #compose #file-ownership

## Components (what exists)
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) to learn about: Apps are identified by folder name; service hostnames derive from app name + service name unless overridden. #scotty #apps #vocabulary
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) to learn about: Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees. #scotty #apps #vocabulary
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) to learn about: App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`. #authorization #scopes #config
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) to learn about: Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands. #blueprints #configuration #env
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) to learn about: Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions. #blueprints #map
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) to learn about: run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes. #custom-actions #authorization #blueprints
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) to learn about: Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions. #custom-actions #workflow #security
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) to learn about: Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError. #logs #docker #websocket #frontend
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) to learn about: Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error. #logs #docker #websocket
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) to learn about: App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever. #state-machine #tasks #docker

## By topic

### #scotty
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #docker
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Each app gets its own dedicated Traefik proxy network**](../traefik/map-traefik-per-app-proxy-network.md) — Scotty creates a per-app network (<network>--<app-name>) instead of one shared network, to avoid Docker DNS alias collisions.
### #blueprints
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
### #apps
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../auth/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](../auth/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #compose
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #custom-actions
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
### #frontend
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
- Open [**Frontend tooling uses bun, not npm**](../frontend/practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #logs
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #vocabulary
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #websocket
- Open [**Follow mode is a no-op notice, not an error, on stopped containers**](map-follow-mode-unavailable-for-stopped-containers.md) — Requesting live log follow on a stopped container returns historical logs plus an informational notice and a clean stream end, not LogsStreamError.
- Open [**Log streaming behavior for stopped vs missing containers**](map-container-log-streaming-for-stopped-containers.md) — Stopped/exited containers return retained historical logs instead of an error; only a truly missing container is an error.
### #workflow
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](../workflow/practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](../workflow/practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.
### #cli
- Open [**app:cp path-spec parsing and pipe-mode semantics**](../cli/practice-app-cp-path-spec-and-pipe-mode.md) — app:cp resolves remote/local/stdio arguments docker-cp-style, resolves omitted service names from the blueprint, and treats pipe mode as single-file tar with lossy metadata.
- Open [**scottyctl app:cp moves files between workstation and app containers**](../cli/map-scottyctl-app-cp.md) — CLI subcommand to copy files in/out of a service container, supports stdin/stdout piping.
- Open [**scottyctl CLI namespace and behavior**](../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #config
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](../configuration/practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](../configuration/map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #container
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #env
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**.env / .env.local precedence and usage convention**](../configuration/practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
### #file-ownership
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #gotcha
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../auth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #map
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #restriction
- Open [**compose.yml must not expose ports directly or use env-var expansion**](practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #scopes
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../auth/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../auth/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #state-machine
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.
### #status
- Open [**Running status treats clean one-shot exits as completed, gates URLs per-service**](practice-container-status-one-shot-completion.md) — App status aggregation distinguishes a clean Exited(0) one-shot container from a crash, and the frontend shows a service's URL based on that service's own status rather than the aggregate app status.
### #tasks
- Open [**State machine handler errors always mark the task Failed**](map-state-machine-errors-bubble-to-task-failure.md) — App lifecycle state machines propagate handler errors (and panics) up through spawn() so the owning task is always marked Failed instead of hanging forever.