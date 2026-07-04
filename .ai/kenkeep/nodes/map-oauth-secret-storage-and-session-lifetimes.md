---
type: map
title: OAuth PKCE/CSRF secrets use MaskedSecret; session lifetimes are short-lived
description: >-
  PKCE verifiers and CSRF tokens are stored via the MaskedSecret type (zeroized,
  log/memory-dump protected); OAuth sessions expire in 5 minutes, web flow
  sessions in 10, cleanup runs every 5 minutes.
tags:
  - oauth
  - security
  - sessions
kk_schema_version: 3
kk_id: map-oauth-secret-storage-and-session-lifetimes
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
PKCE verifiers and CSRF tokens generated during the OAuth flow are stored in memory using the `MaskedSecret` type, which protects them from memory dumps and logs and zeroizes them automatically. The PKCE verifier is single-use and removed from the session store immediately after a successful token exchange.

OAuth sessions expire 5 minutes after token exchange; web flow sessions expire after 10 minutes. A background task cleans up expired sessions every 5 minutes. The CSRF state parameter combines the session ID and CSRF token in a `session_id:csrf_token` format.
