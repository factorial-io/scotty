---
type: map
title: >-
  Blueprint action scripts get SCOTTY__APP_NAME and
  SCOTTY__PUBLIC_URL__<SERVICE> injected
description: >-
  Scotty auto-injects the app name and each public service's URL as env vars
  into blueprint action commands.
tags:
  - blueprints
  - configuration
  - env
kk_schema_version: 3
kk_id: map-blueprint-injected-env-vars
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When blueprint action scripts (`post_create`, `post_rebuild`, `post_run`, `post_destroy`) run, Scotty automatically injects `SCOTTY__APP_NAME` (the app's name) and `SCOTTY__PUBLIC_URL__<SERVICE_NAME>` for each service with a configured public URL — the service name is sanitized into a valid env var name (e.g. `my-service` → `SCOTTY__PUBLIC_URL__MY_SERVICE`).

These are available alongside any env vars configured for the app via `--env` or `.scotty.yml`, and are commonly used in blueprint commands like `drush uli --uri="$SCOTTY__PUBLIC_URL__NGINX"`.
