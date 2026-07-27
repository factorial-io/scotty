---
type: practice
title: Bearer tokens must be explicitly assigned in authorization policy
description: >-
  Bearer token auth has no legacy fallback; unassigned tokens get 401, not
  api.access_token.
tags:
  - authorization
  - bearer-token
  - casbin
  - auth
kk_schema_version: 3
kk_id: practice-bearer-tokens-require-rbac-assignment
kk_derived_from: []
kk_relates_to:
  - practice-access-token-config-removed-use-bearer-tokens
kk_depends_on: []
kk_confidence: high
---
The authorization system requires explicit bearer token assignments: only tokens explicitly listed in the `assignments` section of `config/casbin/policy.yaml` are accepted. Tokens not listed there are rejected with a 401 Unauthorized response — the legacy `api.access_token` setting is never consulted in this path.

One exception: when the Casbin config itself fails to load, Scotty degrades to a fallback authorization service that grants a configured `api.access_token` an admin role on the `default` scope (see the related legacy access-token node). With a working policy file, that path is never taken.
