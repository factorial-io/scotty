---
type: practice
title: Scotty only writes compose.override.yml and .scotty.yml in an app directory
description: >-
  Scotty must not modify any file in an app's directory besides
  compose.override.yml and .scotty.yml.
tags:
  - scotty
  - compose
  - file-ownership
kk_schema_version: 3
kk_id: practice-scotty-only-touches-override-and-settings-files
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty reads `compose.yml` and, if present, `.scotty.yml` from each app directory it discovers. It uses `.scotty.yml` to generate a `compose.override.yml` that tells the load balancer how to reach the app's exposed services. Scotty does not touch any other file in the directory besides `compose.override.yml` and `.scotty.yml`.
