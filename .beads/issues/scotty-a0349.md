---
title: Consider DashMap for WebSocket client management
status: open
priority: 1
issue_type: task
labels:
- performance
- websocket
created_at: 2025-10-26T20:21:10.680572+00:00
updated_at: 2025-11-24T20:17:25.568029+00:00
---

# Description

WebSocket client HashMap uses Arc&lt;Mutex&lt;HashMap&gt;&gt; which causes lock contention during broadcasts. DashMap provides lock-free reads.

# Design

Location: scotty/src/api/websocket/messaging.rs

Current implementation uses mutex-protected HashMap which can cause contention during broadcasts to multiple clients.

Proposed solution:
```rust
use dashmap::DashMap;

pub struct WebSocketMessenger {
    clients: Arc&lt;DashMap&lt;Uuid, WebSocketClient&gt;&gt;,
}
```

Benefits:
- Lock-free concurrent reads
- Automatic sharding for better concurrency
- Reduced contention during broadcasts

Impact: Reduced lock contention during broadcasts
Effort: 2-3 hours
