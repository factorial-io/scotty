---
title: Implement token caching for OIDC validation
status: open
priority: 2
issue_type: task
labels:
- caching
- oauth
- performance
created_at: 2025-10-27T10:42:30.587922+00:00
updated_at: 2025-11-24T20:17:25.587336+00:00
---

# Description

OIDC token validation makes an HTTP request to /oauth/userinfo on every API request, causing high latency, increased load on OIDC provider, potential rate limiting, and poor API performance.

# Design

Location: scotty/src/oauth/device_flow.rs:221-256

Current code makes HTTP call on EVERY validation. 

Recommendation: Implement in-memory token cache with TTL using moka or mini-moka crate for built-in TTL and LRU eviction.

⚠️ CRITICAL: Must implement cache entry cleanup to prevent memory leaks:
1. Background task to remove expired entries (every 5-10 minutes)
2. Max cache size with LRU eviction policy
3. Manual invalidation on token revocation events

Performance Impact:
- Reduces validation latency from ~100-500ms to <1ms (cache hit)
- Reduces OIDC provider load by 95%+
- Improves API response times significantly

Related: scotty-25683 for cache hit/miss metrics
