---
title: Frontend container log viewer UI
status: closed
priority: 2
issue_type: feature
depends_on:
  scotty-541fa: parent-child
created_at: 2025-10-24T22:58:21.905260+00:00
updated_at: 2025-11-24T20:17:25.571039+00:00
closed_at: 2025-10-24T23:06:56.395577+00:00
---

# Description

Add UI to view container logs in the web frontend. Backend log streaming API is complete with WebSocket support, but frontend has no log viewer component.

# Design

- Create log viewer component similar to unified-output.svelte
- Add WebSocket handlers for LogLineReceived/LogStreamStarted/LogStreamEnded messages
- Add log viewer page or modal accessible from app detail page
- Support follow mode, timestamps, line limits
- Reuse webSocketStore.ts infrastructure

# Acceptance Criteria

- Can view historical logs for any service
- Follow mode for real-time streaming
- Toggle timestamps on/off
- Auto-scroll control
- Copy logs to clipboard
- Integration from app detail page
