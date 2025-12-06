---
title: Add test for auth:status with invalid OAuth token
status: open
priority: 3
issue_type: task
depends_on:
  scotty-0791a: blocks
created_at: 2025-12-06T14:28:46.767793+00:00
updated_at: 2025-12-06T14:35:45.400610+00:00
---

# Description

Location: scottyctl/tests/ (create new test file or add to existing)

Test scenarios:
1. Valid OAuth token - should exit with code 0
2. Expired/invalid OAuth token (401 response) - should exit with code 1
3. Valid Bearer token - should exit with code 0  
4. Invalid Bearer token (401/403 response) - should exit with code 1
5. No auth - existing behavior (should exit with code 0, shows 'not authenticated')

Use wiremock to mock server responses
Test both exit code and error message content
Verify error messages suggest appropriate re-auth method (OAuth vs Bearer)

Part of: scotty-28453
