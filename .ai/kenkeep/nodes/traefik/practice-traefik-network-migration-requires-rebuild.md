---
type: practice
title: 'Migrating an app from the old shared Traefik network requires app:rebuild'
description: >-
  Apps created before the per-app-network change keep using the old shared
  network until you run app:rebuild; app:run does not migrate them.
tags:
  - traefik
  - docker
  - networking
  - migration
kk_schema_version: 3
kk_id: practice-traefik-network-migration-requires-rebuild
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Apps created before the per-app proxy network change still have a `docker-compose.override.yml` referencing the old shared network. They keep running and stay routable on that network until migrated — Scotty does not rewrite the override automatically.

Run `app:rebuild` on each existing app to regenerate the override onto its per-app network and reconnect Traefik. A plain `app:run` does not rewrite the override, so `app:rebuild` is required as the migration step.
