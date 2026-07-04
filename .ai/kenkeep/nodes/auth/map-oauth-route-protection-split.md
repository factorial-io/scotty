---
type: map
title: Public vs protected routes under OAuth mode
description: >-
  Scotty's route protection split: which paths are public and which require
  authentication.
tags:
  - oauth
  - auth
  - routing
  - security
kk_schema_version: 3
kk_id: map-oauth-route-protection-split
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Under OAuth mode, `/`, `/api/v1/health`, `/api/v1/info`, `/api/v1/login`, `/oauth/*`, static assets, and SPA routes are public. Everything under `/api/v1/authenticated/*` (all state-modifying API operations) is protected and requires authentication.

New endpoints should be placed under `/api/v1/authenticated/*` unless they are deliberately meant to be reachable without auth, matching this existing split.
