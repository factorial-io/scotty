---
# scotty-x24f
title: Add test for auth:status with invalid OAuth token
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  Location: scottyctl/tests/ (create new test file or add to existing)  Test scenarios: 1. Valid OAuth token - should exit with code 0 2. Expired/invalid OAuth token (401 response) - should exit with code 1 3. Valid Bearer token - should exit with code 0   4. Invalid Bearer token (401/403 response) - should exit with code 1 5. No auth - existing behavior (should exit with code 0, shows 'not authenticated')  Use wiremock to mock server responses Test both exit code and error message content Verify error messages suggest appropriate re-auth method (OAuth vs Bearer)  Part of: scotty-28453
