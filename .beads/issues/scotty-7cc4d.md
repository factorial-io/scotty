---
title: 'Optimize auth flow: check bearer tokens before OAuth'
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-b5863: blocks
created_at: 2025-11-03T10:22:43.976607+00:00
updated_at: 2025-11-24T20:17:25.565897+00:00
closed_at: 2025-11-03T13:03:05.803754+00:00
---

# Description

Current implementation tries OAuth first (network call to OIDC provider) then falls back to bearer tokens. This adds unnecessary latency for service accounts that will always use bearer tokens. We should check if the token exists in configured bearer_tokens first (fast HashMap lookup), and only try OAuth if not found.

# Design

Update scotty/src/api/basic_auth.rs OAuth mode logic:

Current (slow for service accounts):
1. Try OAuth (network call) → fail → try bearer token

Optimized (fast for service accounts):
1. Extract token from Authorization header
2. Check if token exists in bearer_tokens config (use find_token_identifier)
3. If found → authenticate as bearer token
4. If not found → try OAuth validation

Implementation:
- Add helper function or inline check for bearer token existence
- Reorder authentication attempts
- Add log messages for which path was taken
- Maintain constant-time comparison for security
- Add tests verifying the optimization works

# Acceptance Criteria

- Bearer tokens checked before OAuth (no network call for service accounts)
- OAuth still works for non-bearer tokens
- Log messages indicate which auth path was used
- Tests verify bearer tokens bypass OAuth attempt
- Performance improvement measurable (no OIDC provider latency for service accounts)
- Security maintained (constant-time comparison)
