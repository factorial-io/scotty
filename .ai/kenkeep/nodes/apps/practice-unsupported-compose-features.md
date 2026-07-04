---
type: practice
title: compose.yml must not expose ports directly or use env-var expansion
description: >-
  Scotty marks apps unsupported if compose.yml exposes ports directly or uses
  environment-variable expansion.
tags:
  - scotty
  - compose
  - restriction
kk_schema_version: 3
kk_id: practice-unsupported-compose-features
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty does not support all possible docker-compose settings. Two features make an app unsupported: exposing ports directly (this might conflict with other running apps), and using environment-variable expansion inside `compose.yml` (Scotty can't know the values at runtime). Such apps can be adopted manually with the environment variable values provided in `.scotty.yml` instead.
