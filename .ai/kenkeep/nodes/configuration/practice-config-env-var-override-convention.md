---
type: practice
title: 'Config keys are overridden via SCOTTY__ prefixed, double-underscore env vars'
description: >-
  Any config.yaml key can be overridden by an env var: prefix SCOTTY__, replace
  dots/nesting with double underscores.
tags:
  - configuration
  - env
kk_schema_version: 3
kk_id: practice-config-env-var-override-convention
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The default configuration lives in `config/default.yaml` and can be overridden by `config/local.yaml` or by environment variables — the latter is preferred for sensitive data.

Rule of thumb: to override a nested key, replace the dots with double underscores and prefix with `SCOTTY__`. For example `api.bearer_tokens.admin` becomes `SCOTTY__API__BEARER_TOKENS__ADMIN`. Scotty prints the resolved configuration on startup so overrides can be verified.
