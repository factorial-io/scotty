---
type: practice
title: api.access_token is legacy — only honored in the Casbin fallback path
description: >-
  api.access_token still exists but is only used when the Casbin config fails
  to load, where it grants admin on the default scope; use api.bearer_tokens.
tags:
  - auth
  - configuration
  - gotcha
kk_schema_version: 3
kk_id: practice-access-token-config-removed-use-bearer-tokens
kk_derived_from: []
kk_relates_to:
  - practice-bearer-tokens-require-rbac-assignment
kk_depends_on: []
kk_confidence: medium
---
The `api.access_token` setting is legacy but still live (`scotty-core/src/settings/api_server.rs`, `access_token: Option<String>`). It is consumed only by the fallback authorization service: when the Casbin config at `config/casbin` fails to load, `FallbackService::create_fallback_service` (`scotty/src/services/authorization/fallback.rs`) assigns the configured token an `admin` role on the `default` scope. In normal operation — Casbin policy loaded — the setting plays no role.

For all regular setups, configure `api.bearer_tokens` instead: a map of logical identifiers to tokens (e.g. `admin`, `deployment`), referenced in authorization policy assignments as `identifier:<name>` and overridable per-token via `SCOTTY__API__BEARER_TOKENS__<NAME>`.
