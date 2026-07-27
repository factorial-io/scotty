---
type: map
title: Public base URL resolves through two chains with different fallbacks
description: >-
  public_base_url() falls back to the localhost default; landing-page own-domain
  detection requires an explicitly configured base URL and fail-safes to serving
  the frontend.
tags:
  - configuration
  - landing-page
  - base-url
kk_schema_version: 3
kk_id: map-public-base-url-resolves-through-two-chains-with-different-fallbacks
kk_derived_from:
  - '08436e22-ac06-4970-a04c-9e39d3d7bc13:map:0'
kk_relates_to:
  - practice-default-backend-configuration
  - practice-oauth-redirect-url-vs-frontend-base-url
kk_depends_on: []
kk_confidence: high
---
`ApiServer::public_base_url()` (`scotty-core/src/settings/api_server.rs`) resolves `api.base_url` → deprecated `api.oauth.frontend_base_url` → `DEFAULT_BASE_URL` (`http://localhost:21342`). That localhost default is used only by consumers where a wrong guess is harmless (e.g. OAuth post-login redirects in dev).

The landing-page path (`scotty/src/api/rest/handlers/landing.rs`) instead calls `configured_base_url()`, which returns `None` when neither setting is explicitly set; `is_scotty_domain()` then treats every request as Scotty's own domain and serves the frontend, logging a one-time warning. Rationale: if an operator forgot to set `base_url` in production, falling back to the localhost default would 302 every app-domain visitor to a URL only valid on the operator's machine — serving the frontend is the fail-safe. Local dev therefore needs `api.base_url` set explicitly (it is set in `config/local.yaml`) for stopped-app landing redirects to work.

<!-- kk:related:start -->
# Related

- Related: [practice-default-backend-configuration](/traefik/practice-default-backend-configuration.md)
- Related: [practice-oauth-redirect-url-vs-frontend-base-url](/auth/practice-oauth-redirect-url-vs-frontend-base-url.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [08436e22-ac06-4970-a04c-9e39d3d7bc13:map:0](08436e22-ac06-4970-a04c-9e39d3d7bc13:map:0)
<!-- kk:citations:end -->
