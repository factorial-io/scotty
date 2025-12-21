---
# scotty-a794
title: Consider DashMap for WebSocket client management
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:37Z
parent: scotty-lbxn
---

# Description  WebSocket client HashMap uses Arc&lt;Mutex&lt;HashMap&gt;&gt; which causes lock contention during broadcasts. DashMap provides lock-free reads.  # Design  Location: scotty/src/api/websocket/messaging.rs  Current implementation uses mutex-protected HashMap which can cause contention during broadcasts to multiple clients.  Proposed solution: ```rust use dashmap::DashMap;  pub struct WebSocketMessenger {     clients: Arc&lt;DashMap&lt;Uuid, WebSocketClient&gt;&gt;, } ```  Benefits: - Lock-free concurrent reads - Automatic sharding for better concurrency - Reduced contention during broadcasts  Impact: Reduced lock contention during broadcasts Effort: 2-3 hours
