---
title: Implement session cleanup for expired OAuth sessions
status: closed
priority: 1
issue_type: bug
assignee: claude
labels:
- memory-leak
- oauth
- security
created_at: 2025-10-27T10:42:30.048154+00:00
updated_at: 2025-11-24T20:17:25.585647+00:00
closed_at: 2025-10-27T14:28:59.581236+00:00
---

# Description

OAuth session storage (DeviceFlowStore, WebFlowStore, OAuthSessionStore) accumulates expired sessions without cleanup, leading to unbounded memory growth.

# Design

Impact:
- Memory Leak: Server memory consumption grows indefinitely
- DoS Vector: Attackers can exhaust server memory by creating sessions
- Production Risk: Requires server restarts to reclaim memory

Current: All three session stores use Arc<Mutex<HashMap<String, Session>>> without expiration cleanup.

Recommendation: Implement periodic cleanup task:
1. Add background task using clokwerk scheduler (already in workspace dependencies)
2. Scan sessions every N minutes
3. Remove entries where SystemTime::now() > expires_at
4. Consider using a TTL cache library (ttl_cache, moka)
