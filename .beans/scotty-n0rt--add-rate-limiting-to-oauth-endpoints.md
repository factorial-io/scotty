---
# scotty-n0rt
title: Add rate limiting to OAuth endpoints
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:50:25Z
---

# Description  OAuth endpoints are publicly accessible without rate limiting, allowing DoS attacks and credential stuffing.  # Design  Vulnerable Endpoints (scotty/src/api/router.rs:415-419): - POST /oauth/device - POST /oauth/device/token   - GET /oauth/authorize - GET /api/oauth/callback - POST /oauth/exchange  Attack Scenarios: 1. Credential Stuffing: Unlimited token validation attempts 2. Session Exhaustion: Create thousands of sessions (combined with scotty-f4956 memory leak) 3. Resource Exhaustion: Flood OIDC validation endpoint 4. Enumeration: Discover valid session IDs through brute force  Recommendation: Implement rate limiting using tower-governor middleware: ```rust use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};  let public_limiter = GovernorConfigBuilder::default()     .per_millisecond(100)  // 10 requests per second     .burst_size(20)     .finish()     .unwrap(); ```  Add dependency: `tower-governor = "0.4"`
