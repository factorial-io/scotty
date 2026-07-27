---
type: practice
title: Never store real bearer tokens in configuration files
description: >-
  Keep only placeholder values for api.bearer_tokens in config files; supply
  real secrets via SCOTTY__API__BEARER_TOKENS__<NAME> env vars.
tags:
  - security
  - auth
  - configuration
  - secrets
kk_schema_version: 3
kk_id: practice-never-store-real-bearer-tokens-in-config
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
`api.bearer_tokens` is required for bearer authentication and maps logical token identifiers to secure bearer tokens. The docs explicitly warn: never store actual bearer tokens in configuration files — use placeholder values (e.g. `"OVERRIDE_VIA_ENV_VAR"`) in `config/local.yaml`/`config/default.yaml` and override them with environment variables like `SCOTTY__API__BEARER_TOKENS__ADMIN`.

Use cryptographically strong generated tokens (e.g. `openssl rand -base64 32`), rotate them periodically (regenerate, update env vars, restart the server, update clients), and apply least-privilege scoping via the authorization system rather than one all-powerful token.
