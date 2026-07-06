# kenkeep Index: auth / rate-limiting

↑ Parent: [auth](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) to learn about: In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N. #rate-limiting #deployment #gotcha

## Components (what exists)
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) to learn about: public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user). #rate-limiting #security #api

## By topic

### #rate-limiting
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
### #api
- Open [**Frontend has no API versioning, evolves in lockstep with backend**](../../frontend/practice-frontend-backend-tight-coupling.md) — Scotty frontend is tightly coupled to the backend API; no versioning or backwards compatibility is maintained, so breaking API changes are acceptable.
- Open [**Rate limiting has three independent tiers keyed differently**](map-rate-limiting-tiers.md) — public_auth and oauth tiers rate-limit by client IP; the authenticated tier rate-limits per bearer token (per-user).
### #deployment
- Open [**Rate limits are per-instance, not global, across multiple Scotty instances**](practice-rate-limiting-is-per-instance-not-global.md) — In-memory token-bucket rate limiting is per-process; N horizontally-scaled instances multiply the effective limit by N.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](../../configuration/map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #gotcha
- Open [**api.access_token is legacy — only honored in the Casbin fallback path**](../../configuration/practice-access-token-config-removed-use-bearer-tokens.md) — api.access_token still exists but is only used when the Casbin config fails to load, where it grants admin on the default scope; use api.bearer_tokens.
- Open [**apps.root_folder must match the host mount path when Scotty runs in Docker**](../../configuration/practice-root-folder-must-match-docker-mount-path.md) — If Scotty runs containerized, the apps root_folder path inside the container must equal the host path, or docker-compose fails to run apps.
- Open [**OAuth config has two distinct URLs that must not be confused**](../oauth/practice-oauth-redirect-url-vs-frontend-base-url.md) — redirect_url is the backend's OAuth callback (must match the OIDC provider's app config); frontend_base_url is the frontend's base URL Scotty redirects users back to.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.