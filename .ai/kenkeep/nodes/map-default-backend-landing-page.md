---
type: map
title: Scotty as Traefik default backend / landing page
description: >-
  Scotty can serve as the load balancer's catch-all backend, showing a Start-app
  landing page for stopped apps instead of a gateway error.
tags:
  - landing-page
  - traefik
  - load-balancer
  - architecture
kk_schema_version: 3
kk_id: map-default-backend-landing-page
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When a user visits the domain of a stopped app, Traefik has no route for it, so a catch-all router forwards the request to Scotty acting as the default backend. Scotty inspects the Host header, finds the owning app, and 302-redirects to its own landing page (`/landing/<app>?return_url=...`). The landing page shows three states: Stopped (Start App button), Starting (live docker-compose output over WebSocket), and Ready (countdown then redirect back to the original URL). Login happens invisibly via OAuth between clicking Start and seeing startup output, auto-triggered on return via an `autostart=true` parameter.

Scotty responds differently depending on app status: `Stopped` gets a 302 to the landing page; `Running` gets a 503 with `Retry-After: 5` (routing hasn't caught up, also logged as a possible misconfiguration); any other status (starting/creating) gets a 503 "application is starting up"; a domain belonging to no known app gets 404; requests for Scotty's own domain serve the normal frontend. All landing-related responses are sent with `Cache-Control: no-store` so redirects and error pages for stopped apps are never cached.
