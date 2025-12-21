---
# scotty-xct5
title: Implement session cleanup for expired OAuth sessions
status: completed
type: bug
priority: critical
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:50:24Z
blocking:
    - scotty-qmip
---

# Description  OAuth session storage (DeviceFlowStore, WebFlowStore, OAuthSessionStore) accumulates expired sessions without cleanup, leading to unbounded memory growth.  # Design  Impact: - Memory Leak: Server memory consumption grows indefinitely - DoS Vector: Attackers can exhaust server memory by creating sessions - Production Risk: Requires server restarts to reclaim memory  Current: All three session stores use Arc<Mutex<HashMap<String, Session>>> without expiration cleanup.  Recommendation: Implement periodic cleanup task: 1. Add background task using clokwerk scheduler (already in workspace dependencies) 2. Scan sessions every N minutes 3. Remove entries where SystemTime::now() > expires_at 4. Consider using a TTL cache library (ttl_cache, moka)
