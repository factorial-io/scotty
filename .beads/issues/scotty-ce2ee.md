---
title: Fix hardcoded localhost in OAuth callback URL
status: closed
priority: 1
issue_type: bug
labels:
- oauth
- production-blocker
- security
created_at: 2025-10-27T10:42:29.942070+00:00
updated_at: 2025-11-24T20:17:25.574411+00:00
closed_at: 2025-10-27T14:02:23.208743+00:00
---

# Description

The OAuth callback URL is hardcoded to use localhost, which breaks OAuth flows when the server is accessed via different hostnames.

# Design

Location: scotty/src/oauth/handlers.rs:456-459

Current code hardcodes localhost:
```rust
format!(
    "http://localhost:21342/oauth/callback?session_id={}",
    oauth_session_id
)
```

Recommendation: Add a `frontend_base_url` configuration option to allow dynamic callback URL construction:
- Extract from request Host header
- Fall back to configured base URL
- Support both HTTP and HTTPS schemes

Impact: OAuth web flows fail when scotty is accessed through production domains
