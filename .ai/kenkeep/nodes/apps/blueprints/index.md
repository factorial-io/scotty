# kenkeep Index: apps / blueprints

↑ Parent: [apps](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
_None yet._

## Components (what exists)
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) to learn about: Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands. #blueprints #configuration #env
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) to learn about: Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions. #blueprints #map

## By topic

### #blueprints
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](../custom-actions/map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**OAuth config has two distinct URLs that must not be confused**](../../auth/oauth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #env
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**.env / .env.local precedence and usage convention**](../../configuration/practice-dotenv-precedence-scotty.md) — Scotty auto-loads .env and .env.local; env vars > .env.local > .env, with .env committable and .env.local gitignored.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
### #map
- Open [**Blueprints are reusable app templates**](map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](../../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](../../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.