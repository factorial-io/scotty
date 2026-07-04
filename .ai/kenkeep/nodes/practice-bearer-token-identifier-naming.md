---
type: practice
title: >-
  Bearer token identifiers vs secret values; identifier: prefix for service
  accounts
description: >-
  policy.yaml assignments reference identifiers, not secret token values;
  service accounts use an identifier: prefix, OAuth users use their email.
tags:
  - auth
  - casbin
  - bearer-tokens
  - naming
kk_schema_version: 3
kk_id: practice-bearer-token-identifier-naming
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty separates identifiers from secrets: `bearer_tokens` (in `default.yaml` or via `SCOTTY__API__BEARER_TOKENS__*` env vars) maps a human-readable identifier to the actual secret token value. `policy.yaml` assignments only ever reference the identifier, never the token value itself. On each request, Scotty looks up the presented bearer token in `bearer_tokens` to resolve it to an identifier, then checks that identifier's role/scope assignment in `policy.yaml`.

Naming convention: OAuth users are identified in `policy.yaml` by the email address from their OIDC token (e.g. `admin@example.com`). Service accounts use an `identifier:` prefix with a semantic name (e.g. `identifier:ci-bot:`). Avoid naming identifiers after the token value itself or using confusing names like `identifier:test-bearer-token-123:`.
