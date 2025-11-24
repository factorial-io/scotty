---
title: Protect PKCE verifier with secrecy wrapper
status: open
priority: 2
issue_type: task
labels:
- oauth
- security
created_at: 2025-10-27T10:42:30.152917+00:00
updated_at: 2025-11-24T20:17:25.567379+00:00
---

# Description

The PKCE verifier in WebFlowSession is stored as a plain String, allowing it to be logged, dumped in stack traces, or accidentally exposed.

# Design

Location: scotty/src/oauth/mod.rs:40-48

Current code stores PKCE verifier as plain String. This may appear in logs or error messages.

Recommendation: Use the secrecy crate (already in workspace dependencies):
```rust
use scotty_core::utils::secret::MaskedSecret;

pub struct WebFlowSession {
    pub csrf_token: String,
    pub pkce_verifier: MaskedSecret<String>,  // Protected
    pub redirect_url: String,
    pub frontend_callback_url: Option<String>,
    pub expires_at: SystemTime,
}
```

The project already has MaskedSecret wrapper in scotty-core/src/utils/secret.rs.
