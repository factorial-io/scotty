---
type: practice
title: 'In hybrid OAuth+bearer mode, bearer tokens are checked before OAuth'
description: >-
  Authentication middleware checks bearer tokens (fast HashMap lookup) first,
  then falls back to OAuth (network call), so service accounts pay no OAuth
  latency.
tags:
  - oauth
  - bearer-tokens
  - auth
  - performance
kk_schema_version: 3
kk_id: practice-hybrid-auth-bearer-checked-first
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
When `auth_mode` is `oauth` and `bearer_tokens` are also configured, authentication extracts the token from the `Authorization: Bearer <token>` header and checks it against the bearer token HashMap first. Only if that lookup fails does Scotty fall back to OAuth validation, which requires a network call to the OIDC provider.

This ordering is deliberate: it ensures service accounts (CI/CD, monitoring, automation) experience zero OAuth latency, while human users still authenticate via OAuth. Preserve this check order if the authentication middleware is ever modified.
