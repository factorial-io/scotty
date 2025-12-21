---
# scotty-qmip
title: Add OAuth metrics for monitoring
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  OAuth flows lack observability metrics, making it difficult to monitor authentication health, detect attack patterns, track session lifecycle, and debug production issues.  # Design  Add OAuth-specific metrics to scotty/src/metrics/instruments.rs:  ```rust // OAuth flow metrics pub oauth_device_flows_active: Gauge<i64>, pub oauth_device_flows_total: Counter<u64>, pub oauth_web_flows_active: Gauge<i64>, pub oauth_web_flows_total: Counter<u64>, pub oauth_flow_duration: Histogram<f64>, pub oauth_flow_failures: Counter<u64>,  // Token metrics pub oauth_token_validations_total: Counter<u64>, pub oauth_token_validation_duration: Histogram<f64>, pub oauth_token_validation_failures: Counter<u64>, pub oauth_active_sessions: Gauge<i64>, pub oauth_expired_sessions_cleaned: Counter<u64>, ```  Instrumentation Points: 1. start_device_flow() - increment device flows total/active 2. start_authorization_flow() - increment web flows total/active 3. validate_oidc_token() - track validation count and duration 4. Session cleanup task - track expired session removal 5. exchange_session_for_token() - track successful exchanges  Related to scotty-f4956 (session cleanup) and scotty-8bf10 (cache metrics)
