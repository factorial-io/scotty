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

Traefik's membership in those networks is **reconciled, not only set at deploy time**. The attachment lives in Docker container state rather than in `traefik-compose.yml`, so recreating the Traefik container would otherwise take every deployed app offline silently — containers stay `Up`, TLS still terminates, and requests just hang. `docker/loadbalancer/network_reconciler.rs` therefore reconnects Traefik to the proxy network of every running app on each running-app check, and (when `traefik.watch_docker_events` is enabled, the default) within seconds of the Traefik container starting. The same pass removes proxy networks whose app no longer exists, guarded on the network having no non-Traefik endpoint attached, and records each app's observed state in `AppData::load_balancer_connectivity` for the UI, `scottyctl app:info`, and the `scotty_traefik_*` metrics.
