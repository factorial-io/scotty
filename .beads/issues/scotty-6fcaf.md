---
title: Instrument WebSocket connections with metrics
status: closed
priority: 2
issue_type: task
depends_on:
  scotty-06fec: blocks
  scotty-6feea: parent-child
created_at: 2025-10-24T23:28:16.086525+00:00
updated_at: 2025-11-24T20:17:25.555949+00:00
closed_at: 2025-10-26T16:38:41.091932+00:00
---

# Description

Add metrics to WebSocket client management for connection counts, message throughput, and authentication failures.

# Design

Instrument WebSocket layer:
- Track websocket_connections_active
- Count messages sent/received
- Track authentication failures
- Monitor disconnect events

# Acceptance Criteria

- Connection lifecycle tracked
- Message counters increment correctly
- Auth failure tracking works
- No overhead on message path
