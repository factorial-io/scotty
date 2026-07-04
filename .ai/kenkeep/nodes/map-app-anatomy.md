---
type: map
title: An app is any folder in the apps directory containing compose.yml
description: >-
  Apps are identified by folder name; service hostnames derive from app name +
  service name unless overridden.
tags:
  - scotty
  - apps
  - vocabulary
kk_schema_version: 3
kk_id: map-app-anatomy
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Every folder with a `compose.yml` file, located inside Scotty's configured apps directory, is considered an app. The app's unique name derives from its folder name and is used to identify it in the UI/CLI. Services defined in `compose.yml` each get a unique hostname derived from the app name and service name, which can be overridden in `.scotty.yml` or at app-creation time. Docker-compose files ideally reference pre-built images but can also build from Dockerfiles.
