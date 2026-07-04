---
type: practice
title: api.access_token is no longer supported — use api.bearer_tokens
description: >-
  The old single api.access_token setting was removed; configure
  api.bearer_tokens (a map of named tokens) instead.
tags:
  - auth
  - configuration
  - gotcha
kk_schema_version: 3
kk_id: practice-access-token-config-removed-use-bearer-tokens
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
The `api.access_token` configuration key is no longer supported. Use `api.bearer_tokens` instead, which is a map of logical identifiers to tokens (e.g. `admin`, `deployment`), referenced in authorization policy assignments as `identifier:<name>` and overridable per-token via `SCOTTY__API__BEARER_TOKENS__<NAME>`.
