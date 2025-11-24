---
title: Improve authorization error handling in Casbin enforcement
status: open
priority: 1
issue_type: task
labels:
- observability
- security
created_at: 2025-10-26T20:21:10.471975+00:00
updated_at: 2025-11-24T20:17:25.556650+00:00
---

# Description

Authorization service silently denies access on Casbin errors using unwrap_or(false), potentially hiding system issues.

# Design

Location: scotty/src/services/authorization/service.rs:182

Current code:
```rust
let result = enforcer
    .enforce(vec![user, app, action_str])
    .unwrap_or(false);
```

Proposed solution:
```rust
let result = enforcer
    .enforce(vec![user, app, action_str])
    .map_err(|e| {
        error!("Casbin enforce error: {}", e);
        crate::metrics::authorization::record_error();
    })
    .unwrap_or(false);
```

Impact: Better visibility into authorization failures
Effort: 30 minutes
