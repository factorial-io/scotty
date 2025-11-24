---
title: Add OAuth metrics for monitoring
status: closed
priority: 2
issue_type: task
assignee: claude
labels:
- metrics
- oauth
- observability
depends_on:
  scotty-f4956: blocks
created_at: 2025-10-27T10:42:30.477735+00:00
updated_at: 2025-11-24T20:17:25.550894+00:00
closed_at: 2025-10-27T14:36:55.309530+00:00
---

# Description

OAuth flows lack observability metrics, making it difficult to monitor authentication health, detect attack patterns, track session lifecycle, and debug production issues.

# Design

Add OAuth-specific metrics to scotty/src/metrics/instruments.rs:

```rust
// OAuth flow metrics
pub oauth_device_flows_active: Gauge<i64>,
pub oauth_device_flows_total: Counter<u64>,
pub oauth_web_flows_active: Gauge<i64>,
pub oauth_web_flows_total: Counter<u64>,
pub oauth_flow_duration: Histogram<f64>,
pub oauth_flow_failures: Counter<u64>,

// Token metrics
pub oauth_token_validations_total: Counter<u64>,
pub oauth_token_validation_duration: Histogram<f64>,
pub oauth_token_validation_failures: Counter<u64>,
pub oauth_active_sessions: Gauge<i64>,
pub oauth_expired_sessions_cleaned: Counter<u64>,
```

Instrumentation Points:
1. start_device_flow() - increment device flows total/active
2. start_authorization_flow() - increment web flows total/active
3. validate_oidc_token() - track validation count and duration
4. Session cleanup task - track expired session removal
5. exchange_session_for_token() - track successful exchanges

Related to scotty-f4956 (session cleanup) and scotty-8bf10 (cache metrics)
