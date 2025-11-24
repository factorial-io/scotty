---
title: Upgrade oauth2 from 4.4 to 5.0
status: open
priority: 3
issue_type: chore
labels:
- dependencies
- oauth
- security
created_at: 2025-10-26T21:08:00.856298+00:00
updated_at: 2025-11-24T20:17:25.582425+00:00
---

# Description

oauth2 crate has a major version update available (4.4 → 5.0). This affects OAuth2 authentication flow in the API.

# Design

Location: scotty/Cargo.toml:61

Current: oauth2 = "4.4"
Target: oauth2 = "5.0"

Steps:
1. Review oauth2 5.0 changelog and migration guide
2. Identify breaking changes in API
3. Update version in scotty/Cargo.toml
4. Update OAuth2 implementation code to match new API
5. Test OAuth2 authentication flows thoroughly
6. Verify token generation and validation still works

Impact: May require changes to OAuth2 authentication implementation
Effort: 3-5 hours
