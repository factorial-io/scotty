---
title: Auto-compute OAuth redirect_url from frontend_base_url
status: open
priority: 2
issue_type: feature
created_at: 2025-11-13T22:18:40.080766+00:00
updated_at: 2025-11-24T20:17:25.581859+00:00
---

# Description

Make redirect_url optional and auto-compute from frontend_base_url during settings initialization. This simplifies OAuth configuration by requiring users to only configure one base URL instead of two redundant URLs.

# Design

**Implementation Approach:**

1. **Update OAuthSettings** (`scotty-core/src/settings/api_server.rs`):
   - Make `redirect_url: Option<String>` instead of `String`
   - Update default to return `None`
   - Add helper method to compute absolute redirect URL

2. **Update Settings initialization** (`scotty/src/settings/config.rs`):
   - After deserializing settings, compute `redirect_url` if not set or relative
   - Use formula: `redirect_url = format!("{}/api/oauth/callback", frontend_base_url)`

3. **Validation**:
   - Ensure computed URL is absolute
   - Validate frontend_base_url is absolute before computing
   - Log computed redirect_url for debugging

**Changes:**
- `scotty-core/src/settings/api_server.rs`: Make redirect_url optional
- `scotty/src/settings/config.rs`: Add redirect_url computation after deserialization
- Update documentation to reflect simplified configuration

# Acceptance Criteria

- [ ] redirect_url is optional in OAuthSettings struct
- [ ] If redirect_url is None, it's computed as {frontend_base_url}/api/oauth/callback
- [ ] If redirect_url is relative (doesn't start with http:// or https://), it's computed from frontend_base_url
- [ ] Explicit absolute redirect_url in config still works (override)
- [ ] OAuth initialization succeeds with only frontend_base_url configured
- [ ] Existing configs with explicit redirect_url continue to work (backward compatible)
- [ ] Tests added for redirect_url computation logic
- [ ] Documentation updated to show simplified configuration example
