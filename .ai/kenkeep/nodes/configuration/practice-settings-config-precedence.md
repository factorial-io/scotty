---
type: practice
title: Settings load order and secret handling
description: >-
  Config precedence: code defaults, then config files, then SCOTTY__-prefixed
  env vars; use env vars (not config files) for bearer tokens.
tags:
  - configuration
  - settings
  - security
kk_schema_version: 3
kk_id: practice-settings-config-precedence
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Settings are loaded via the `config` crate in this order: 1) defaults in code, 2) config files (YAML/TOML), 3) env vars (prefix `SCOTTY__`).

Server bearer tokens should be set via env vars (`SCOTTY__API__BEARER_TOKENS__<NAME>`), not config files. Other notable server env vars: `SCOTTY__API__AUTH_MODE=dev` (disable auth), `SCOTTY__TELEMETRY=metrics,traces`. scottyctl reads `SCOTTY_SERVER` (default `http://localhost:21342`) and `SCOTTY_ACCESS_TOKEN`.
