---
type: practice
title: 'Run app:rebuild after app:adopt to actually enable load balancing'
description: >-
  app:adopt only writes Scotty settings; it does not apply load balancer config
  — app:rebuild is required afterward.
tags:
  - scotty
  - cli
  - workflow
  - gotcha
kk_schema_version: 3
kk_id: practice-adopt-then-rebuild-for-unsupported-apps
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
`app:adopt` only creates the Scotty settings so an app becomes recognized by Scotty; it does not apply the load balancer configuration. Run `app:rebuild` afterward to write the `docker-compose.override.yml`, create the per-app proxy network, and get the app fully working behind Traefik.
