---
# scotty-amct
title: Upgrade oauth2 from 4.4 to 5.0
status: todo
type: task
priority: normal
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:30Z
parent: scotty-8tep
---

# Description  oauth2 crate has a major version update available (4.4 → 5.0). This affects OAuth2 authentication flow in the API.  # Design  Location: scotty/Cargo.toml:61  Current: oauth2 = "4.4" Target: oauth2 = "5.0"  Steps: 1. Review oauth2 5.0 changelog and migration guide 2. Identify breaking changes in API 3. Update version in scotty/Cargo.toml 4. Update OAuth2 implementation code to match new API 5. Test OAuth2 authentication flows thoroughly 6. Verify token generation and validation still works  Impact: May require changes to OAuth2 authentication implementation Effort: 3-5 hours
