---
type: map
title: Rate limiting has three independent tiers keyed differently
description: >-
  public_auth and oauth tiers rate-limit by client IP; the authenticated tier
  rate-limits per bearer token (per-user).
tags:
  - rate-limiting
  - security
  - api
kk_schema_version: 3
kk_id: map-rate-limiting-tiers
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Rate limiting is disabled by default (`api.rate_limiting.enabled: false`) and must be explicitly turned on. It has three independent tiers: `public_auth` (protects `/api/v1/login` from brute force, keyed by client IP), `oauth` (protects OAuth flow endpoints like `/oauth/device`, `/oauth/authorize`, keyed by client IP), and `authenticated` (protects all `/api/v1/authenticated/*` endpoints, keyed per bearer token/user).

Each tier configures its own `requests_per_minute` and `burst_size`. Exceeding a limit returns HTTP 429 with a `Retry-After` header.
