---
type: map
title: 'app:create --registry and --middleware require server-side allow-listing'
description: >-
  Custom registries and middleware referenced by app:create must be
  pre-configured/allow-listed on the server; middleware is Traefik-only.
tags:
  - cli
  - app-create
  - traefik
  - middleware
kk_schema_version: 3
kk_id: map-cli-app-create-registry-middleware-allowlist
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`app:create --registry <REGISTRY>` selects a private registry for pulling images, but the server needs to be configured with that registry beforehand. Similarly, `--middleware <MIDDLEWARE>` (repeatable) attaches middleware to the app, but each middleware name must already be in the load balancer's allow-list in the server configuration before it can be used, and middleware support is currently only implemented for the Traefik backend.
