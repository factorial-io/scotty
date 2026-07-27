---
type: practice
title: Bearer token comparison is constant-time
description: >-
  Bearer token validation uses the subtle crate's constant-time equality instead
  of standard string comparison.
tags:
  - security
  - auth
  - bearer-token
kk_schema_version: 3
kk_id: practice-bearer-token-constant-time-comparison
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Bearer token lookups (both the general auth path and the login handler) compare the presented token against configured tokens with `subtle::ConstantTimeEq` rather than `==`. Standard equality short-circuits on the first byte mismatch, which leaks timing information an attacker could use to reconstruct a valid token byte-by-byte; constant-time comparison closes that side channel.

Any new code path that checks a bearer token against a stored secret must go through the same constant-time comparison rather than a plain string/byte equality check.
