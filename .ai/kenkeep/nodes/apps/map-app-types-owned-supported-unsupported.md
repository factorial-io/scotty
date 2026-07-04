---
type: map
title: 'Apps are categorized as owned, supported, or unsupported'
description: >-
  Scotty validates compose.yml and sorts apps into owned/supported/unsupported,
  each with different lifecycle guarantees.
tags:
  - scotty
  - apps
  - vocabulary
kk_schema_version: 3
kk_id: map-app-types-owned-supported-unsupported
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty validates `compose.yml` and categorizes each app into one of three types:

- **Owned apps**: created by Scotty or manually adopted; Scotty may manage the full lifecycle, including destroying the app and its data.
- **Supported apps**: compose files with no side-effecting settings (no exposed ports, no required env vars); Scotty can handle the full lifecycle but will not allow destroying the app and its data.
- **Unsupported apps**: compose files that expose ports directly or require environment variables to run `docker-compose`. Scotty will not touch these apps but shows them in the UI/CLI; they can be converted via `app:adopt`.
