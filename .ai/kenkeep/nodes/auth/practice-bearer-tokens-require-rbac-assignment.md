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
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
The authorization system requires explicit bearer token assignments: only tokens explicitly listed in the `assignments` section of `config/casbin/policy.yaml` are accepted. The legacy `api.access_token` configuration is no longer used at all.

Bearer tokens that are not explicitly listed in `assignments` are rejected with a 401 Unauthorized response — there is no fallback to legacy configuration. This applies globally, not just during migration from older setups.
