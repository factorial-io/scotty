---
type: practice
title: 'Traefik middlewares: built-ins apply before custom, order matters'
description: >-
  Basic-auth then robots built-ins always precede custom middlewares from
  .scotty.yml, applied in array order; names are case-sensitive.
tags:
  - traefik
  - middleware
  - scotty-yml
  - loadbalancer
kk_schema_version: 3
kk_id: practice-traefik-middleware-ordering
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The `middlewares` array in `.scotty.yml` adds custom Traefik middlewares on top of Scotty's built-ins. Application order is fixed: 1) built-in basic-auth middleware (if `basic_auth` is configured), 2) built-in robots middleware (if `disallow_robots` is true), 3) custom middlewares in the order given in the array.

Middleware names are case-sensitive and must already be defined in the Traefik configuration — if a referenced middleware doesn't exist in Traefik, the service may fail to start. The same middleware name can be shared across apps. Custom middlewares are Traefik-only (see the registry/middleware allow-list node).
