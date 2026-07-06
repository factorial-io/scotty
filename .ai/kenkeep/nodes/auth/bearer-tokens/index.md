# kenkeep Index: auth / bearer-tokens

↑ Parent: [auth](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) to learn about: Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison. #security #auth #bearer-token
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) to learn about: policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email. #auth #casbin #bearer-tokens #naming
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) to learn about: Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency. #oauth #bearer-tokens #auth #performance

## Components (what exists)
_None yet._

## By topic

### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../authorization/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #bearer-tokens
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #bearer-token
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../authorization/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Bearer token comparison is constant-time**](practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](../authorization/practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #naming
- Open [**Bearer token identifiers vs secret values; identifier: prefix for service accounts**](practice-bearer-token-identifier-naming.md) — policy.yaml assignments reference identifiers, not secret token values; service accounts use an identifier: prefix, OAuth users use their email.
### #oauth
- Open [**Public vs protected routes under OAuth mode**](../oauth/map-oauth-route-protection-split.md) — Scotty's route protection split: which paths are public and which require authentication.
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
- Open [**scottyctl: explicit access token wins over cached OAuth token**](../../cli/practice-scottyctl-access-token-precedence.md) — In scottyctl's token resolution, an explicitly supplied --access-token / SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
### #performance
- Open [**In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth**](practice-hybrid-auth-bearer-checked-first.md) — Authentication middleware checks bearer tokens (fast HashMap lookup) first, then falls back to OAuth (network call), so service accounts pay no OAuth latency.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.