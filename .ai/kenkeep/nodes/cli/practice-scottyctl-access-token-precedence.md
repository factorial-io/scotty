---
type: practice
title: 'scottyctl: explicit access token wins over cached OAuth token'
description: >-
  In scottyctl's token resolution, an explicitly supplied --access-token /
  SCOTTY_ACCESS_TOKEN always takes precedence over a cached OAuth token.
tags:
  - scottyctl
  - auth
  - oauth
  - cli
kk_schema_version: 3
kk_id: practice-scottyctl-access-token-precedence
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When scottyctl resolves which credential to send, it checks the explicit access token (from `--access-token` or `SCOTTY_ACCESS_TOKEN`) first and only falls back to a cached OAuth token if none was given. This lets a user override a stale or expired cached OAuth token by passing an explicit token, and makes it possible to use bearer-token auth even when an OAuth token happens to be cached locally.
