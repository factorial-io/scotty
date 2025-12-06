---
title: auth:status should validate OAuth token and return non-zero on invalid token (GH#607)
status: closed
priority: 2
issue_type: bug
created_at: 2025-12-06T14:27:53.561383+00:00
updated_at: 2025-12-06T14:50:23.076121+00:00
closed_at: 2025-12-06T14:50:23.076121+00:00
---

# Description

Currently auth:status reports OAuth is used but doesn't check token validity. Sometimes errors when requesting permissions, sometimes returns silently. Should validate token and return non-zero exit code if invalid.

GitHub Issue: #607
Location: scottyctl/src/commands/auth.rs

## Current Behavior Analysis

In auth:status (auth.rs:146-190):
1. Gets current auth method via get_current_auth_method()
2. For OAuth, simply displays stored token info without validation
3. Calls display_user_permissions() which makes API request to /scopes/list
4. If API call fails with 401/403, prints warning but function still returns Ok(())
5. No non-zero exit code on invalid token

In api.rs:123-146 (get_auth_token):
1. Checks if server supports OAuth
2. Loads stored token from cache
3. TODO comment: 'Check if token is expired and refresh if needed'
4. Uses token without validation

## Root Cause

1. auth:status displays cached token metadata without validating it with server
2. display_user_permissions() swallows auth errors and just prints warnings
3. Function returns Ok(()) even when token is invalid
4. No exit code to indicate failure

## Expected Behavior

1. Validate OAuth token is still valid (make test API call)
2. Return non-zero exit code if token invalid
3. Clear error message to user
4. Suggest re-authentication with auth:login

## Solution Plan

See granular issues for implementation steps.
