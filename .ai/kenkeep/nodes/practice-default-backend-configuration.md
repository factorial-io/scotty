---
type: practice
title: >-
  Default-backend setup requires api.base_url and a low-priority catch-all
  router
description: >-
  Landing-page redirects only work if api.base_url is set and a lowest-priority
  catch-all Traefik router forwards unmatched domains to Scotty.
tags:
  - traefik
  - configuration
  - landing-page
  - gotcha
kk_schema_version: 3
kk_id: practice-default-backend-configuration
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`api.base_url` (or `SCOTTY__API__BASE_URL`) must be set to Scotty's own public URL so it can distinguish its own domain from app domains and build the landing-page redirect target. If unset, Scotty falls back to `api.oauth.frontend_base_url` unless that is still the default `http://localhost:21342`; if neither is configured, Scotty cannot tell its own domain apart from app domains, logs a warning, and serves every request as the frontend — per-app domains will not redirect to the landing page. Always set `api.base_url` in production.

The Traefik catch-all router must use `HostRegexp(`^.+$`)` with `priority=1` (the lowest possible), pointing at the `scotty` service. Running apps' auto-generated routers always have a higher priority (based on rule length), so a running app is never shadowed by the catch-all — only domains with no other route (stopped apps) fall through to it. If Scotty doesn't run as a Traefik-labelled container, the same router/service must be declared via the Traefik file provider instead.
