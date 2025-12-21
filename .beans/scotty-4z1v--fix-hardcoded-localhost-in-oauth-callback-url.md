---
# scotty-4z1v
title: Fix hardcoded localhost in OAuth callback URL
status: completed
type: bug
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  The OAuth callback URL is hardcoded to use localhost, which breaks OAuth flows when the server is accessed via different hostnames.  # Design  Location: scotty/src/oauth/handlers.rs:456-459  Current code hardcodes localhost: ```rust format!(     "http://localhost:21342/oauth/callback?session_id={}",     oauth_session_id ) ```  Recommendation: Add a `frontend_base_url` configuration option to allow dynamic callback URL construction: - Extract from request Host header - Fall back to configured base URL - Support both HTTP and HTTPS schemes  Impact: OAuth web flows fail when scotty is accessed through production domains
