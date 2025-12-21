---
# scotty-gv1j
title: 'scottyctl: access-token should take precedence over cached OAuth tokens'
status: todo
type: bug
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:30Z
parent: scotty-8tep
---

# Description  ## Problem  In scottyctl's get_auth_token() function (scottyctl/src/api.rs:123-146), the authentication precedence order is incorrect:  **Current order:** 1. Cached OAuth token (if server supports OAuth) 2. Explicit access token (from --access-token or SCOTTY_ACCESS_TOKEN)  **Expected order:** 1. Explicit access token (from --access-token or SCOTTY_ACCESS_TOKEN) 2. Cached OAuth token (if server supports OAuth)  ## Why This Matters  - Users explicitly providing an access token expect it to be used immediately - OAuth tokens might be expired or invalid, but user wants to override with fresh token - Violates principle of explicit configuration overriding implicit cached state - Makes it impossible to use bearer tokens when OAuth tokens are cached  ## Location  File: scottyctl/src/api.rs Function: get_auth_token() Lines: 123-146  ## Solution  Swap the order: check server.access_token first (lines 138-141), then fall back to cached OAuth token (lines 131-136) only if no explicit token provided.  ## Related Issues  - scotty-7cc4d: Server-side optimization (different issue, already closed) - scotty-ec021: Test for auth precedence (may need updating after this fix)  ## GitHub Issue  https://github.com/factorial-io/scotty/issues/609
