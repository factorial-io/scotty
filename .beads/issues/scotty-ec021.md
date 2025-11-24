---
title: Add test for OAuth vs bearer token authentication precedence
status: open
priority: 2
issue_type: task
depends_on:
  scotty-b5863: blocks
created_at: 2025-11-03T10:22:44.036978+00:00
updated_at: 2025-11-24T20:17:25.554272+00:00
---

# Description

Add test case to verify that when a token could theoretically be valid for both OAuth and bearer token auth, OAuth takes precedence. This ensures the authentication priority is well-tested and documented.

# Design

Add test in scotty/src/api/bearer_auth_tests.rs:
1. Test name: test_oauth_vs_bearer_precedence
2. Set up scenario where same token value could match both
3. Verify OAuth validation is attempted first
4. Ensure bearer token fallback only happens after OAuth fails
5. Document the precedence behavior in test comments

# Acceptance Criteria

- Test added verifying OAuth takes precedence
- Test passes
- Test comments explain precedence behavior
- Coverage of authentication priority path
