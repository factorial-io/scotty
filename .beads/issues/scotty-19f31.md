---
title: Add rate limiting to OAuth endpoints
status: closed
priority: 1
issue_type: task
labels:
- dos-prevention
- oauth
- security
created_at: 2025-10-27T10:42:30.368145+00:00
updated_at: 2025-11-24T20:17:25.555035+00:00
closed_at: 2025-11-03T20:13:38.408392+00:00
---

# Description

OAuth endpoints are publicly accessible without rate limiting, allowing DoS attacks and credential stuffing.

# Design

Vulnerable Endpoints (scotty/src/api/router.rs:415-419):
- POST /oauth/device
- POST /oauth/device/token  
- GET /oauth/authorize
- GET /api/oauth/callback
- POST /oauth/exchange

Attack Scenarios:
1. Credential Stuffing: Unlimited token validation attempts
2. Session Exhaustion: Create thousands of sessions (combined with scotty-f4956 memory leak)
3. Resource Exhaustion: Flood OIDC validation endpoint
4. Enumeration: Discover valid session IDs through brute force

Recommendation: Implement rate limiting using tower-governor middleware:
```rust
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

let public_limiter = GovernorConfigBuilder::default()
    .per_millisecond(100)  // 10 requests per second
    .burst_size(20)
    .finish()
    .unwrap();
```

Add dependency: `tower-governor = "0.4"`
