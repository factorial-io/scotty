# kenkeep Index: auth / oauth

↑ Parent: [auth](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) to learn about: redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to. #oauth #configuration #gotcha

## Components (what exists)
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) to learn about: PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes. #oauth #security #sessions
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) to learn about: Scotty's route protection split: which paths are public and which require authentication. #oauth #auth #routing #security

## By topic

### #oauth
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](../bearer-tokens/practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
- Open [**scottyctl: explicit access token wins over cached OAuth token**](../../cli/practice-scottyctl-access-token-precedence.md) — In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../authorization/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](../bearer-tokens/practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #configuration
- Open [**Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars**](../../configuration/practice-config-env-var-override-convention.md) — Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace dots/nesting with double underscores.
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #gotcha
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #routing
- Open [**Public vs protected routes under OAuth mode**](map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
### #sessions
- Open [**OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived**](map-oauth-secret-storage-and-session-lifetimes.md) — PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized, log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow sessions in 10, cleanup runs every 5 minutes.