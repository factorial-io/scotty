---
# scotty-6gg2
title: Instrument WebSocket connections with metrics
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  Add metrics to WebSocket client management for connection counts, message throughput, and authentication failures.  # Design  Instrument WebSocket layer: - Track websocket_connections_active - Count messages sent/received - Track authentication failures - Monitor disconnect events  # Acceptance Criteria  - Connection lifecycle tracked - Message counters increment correctly - Auth failure tracking works - No overhead on message path
