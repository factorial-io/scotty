# kenkeep Index: auth

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) to learn about: Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison. #security #auth #bearer-token
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) to learn about: policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email. #auth #casbin #bearer-tokens #naming
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) to learn about: Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token. #authorization #bearer-token #casbin #auth
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) to learn about: Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive. #authorization #casbin #security
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) to learn about: Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency. #oauth #bearer-tokens #auth #performance
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) to learn about: redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to. #oauth #configuration #gotcha
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) to learn about: In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N. #rate-limiting #deployment #gotcha

## Components (what exists)
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) to learn about: RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs. #authorization #casbin #security #map
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) to learn about: Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver. #authorization #rbac #roles #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) to learn about: OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments. #authorization #casbin #auth
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) to learn about: PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes. #oauth #security #sessions
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) to learn about: Scotty's route protection split: which paths are public and which require authentication. #oauth #auth #routing #security
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) to learn about: public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user). #rate-limiting #security #api

## By topic

### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #oauth
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
- Open [**scottyctl: explicit access token wins over cached OAuth token**](../cli/practice-scottyctl-access-token-precedence.md) — In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
### #bearer-token
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #bearer-tokens
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #gotcha
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #rate-limiting
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
### #api
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](../frontend/practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is no longer supported — use api.bearer_tokens**](../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — The old single api.access_token setting was removed; configure api.bearer_tokens (a map of named tokens) instead.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
### #deployment
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](../configuration/map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #map
- Open [**Blueprints are reusable app templates**](../apps/map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #naming
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
### #performance
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #rbac
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.
### #roles
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.
### #routing
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
### #sessions
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) — PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes.