# kenkeep Index: auth / authorization

↑ Parent: [auth](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) to learn about: Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token. #authorization #bearer-token #casbin #auth
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) to learn about: Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive. #authorization #casbin #security

## Components (what exists)
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) to learn about: RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs. #authorization #casbin #security #map
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) to learn about: Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver. #authorization #rbac #roles #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) to learn about: OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments. #authorization #casbin #auth

## By topic

### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #casbin
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
### #auth
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Bearer token comparison is constant-time**](../bearer-tokens/practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #bearer-token
- Open [**Bearer tokens must be explicitly assigned in authorization policy**](practice-bearer-tokens-require-rbac-assignment.md) — Bearer token auth has no legacy fallback; unassigned tokens get 401, not api.access_token.
- Open [**Bearer token comparison is constant-time**](../bearer-tokens/practice-bearer-token-constant-time-comparison.md) — Bearer token validation uses the subtle crate's constant-time equality instead of standard string comparison.
### #map
- Open [**Blueprints are reusable app templates**](../../apps/blueprints/map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Scotty server key modules and their locations**](../../architecture/map-scotty-server-module-map.md) — Map of scotty/src/ modules (api, docker, oauth, onepassword, tasks, notification, metrics) to responsibilities.
- Open [**scottyctl CLI namespace and behavior**](../../cli/map-scottyctl-cli-structure.md) — Colon-namespaced commands, global flags, version preflight check, and gzip+base64 file upload with .scottyignore.
### #rbac
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.
### #roles
- Open [**Built-in authorization roles and their permission sets**](map-authorization-builtin-roles.md) — Named RBAC roles beyond admin/developer/viewer: operator, system_admin, action_approver.