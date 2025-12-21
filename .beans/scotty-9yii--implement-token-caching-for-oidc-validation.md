---
# scotty-9yii
title: Implement token caching for OIDC validation
status: todo
type: task
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T13:33:30Z
parent: scotty-8tep
---

# Description  OIDC token validation makes an HTTP request to /oauth/userinfo on every API request, causing high latency, increased load on OIDC provider, potential rate limiting, and poor API performance.  Location: scotty/src/oauth/device_flow.rs:228-268  Current Issues: - No caching of validated tokens - No retry logic if OIDC provider is temporarily unavailable - No circuit breaker pattern to prevent cascading failures - Each request makes network call to OIDC provider  Required Improvements: - Implement token caching with TTL (respect token expiry) - Add circuit breaker for OIDC provider failures - Consider using tower::retry for transient failures - Cache should invalidate on token expiration  Priority: HIGH - Performance and reliability issue References: PR #467 review from 2025-11-24  # Design  Location: scotty/src/oauth/device_flow.rs:221-256  Current code makes HTTP call on EVERY validation.   Recommendation: Implement in-memory token cache with TTL using moka or mini-moka crate for built-in TTL and LRU eviction.  ⚠️ CRITICAL: Must implement cache entry cleanup to prevent memory leaks: 1. Background task to remove expired entries (every 5-10 minutes) 2. Max cache size with LRU eviction policy 3. Manual invalidation on token revocation events  Performance Impact: - Reduces validation latency from ~100-500ms to <1ms (cache hit) - Reduces OIDC provider load by 95%+ - Improves API response times significantly  Related: scotty-25683 for cache hit/miss metrics
