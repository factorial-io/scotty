---
type: map
title: Each app gets its own dedicated Traefik proxy network
description: >-
  Scotty creates a per-app network (<network>--<app-name>) instead of one shared
  network, to avoid Docker DNS alias collisions.
tags:
  - traefik
  - docker
  - networking
  - loadbalancer
kk_schema_version: 3
kk_id: map-traefik-per-app-proxy-network
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
To avoid Docker DNS name collisions across apps (every app defining an `nginx` service would otherwise publish the same `nginx` alias onto one shared network), Scotty gives each app its own dedicated proxy network instead of a single shared network. For an app named `myapp` and a base `network` of `proxy` (the `traefik.network` config value), the per-app network is `proxy--myapp`.

Scotty creates this network before starting the app, connects the Traefik container to it, and removes it again when the app is destroyed or purged. Public services are tagged with the `traefik.docker.network` label so Traefik knows which network to route over. Users do not need to create these networks manually.
