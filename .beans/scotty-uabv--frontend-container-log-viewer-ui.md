---
# scotty-uabv
title: Frontend container log viewer UI
status: completed
type: feature
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:48Z
parent: scotty-m89i
---

# Description  Add UI to view container logs in the web frontend. Backend log streaming API is complete with WebSocket support, but frontend has no log viewer component.  # Design  - Create log viewer component similar to unified-output.svelte - Add WebSocket handlers for LogLineReceived/LogStreamStarted/LogStreamEnded messages - Add log viewer page or modal accessible from app detail page - Support follow mode, timestamps, line limits - Reuse webSocketStore.ts infrastructure  # Acceptance Criteria  - Can view historical logs for any service - Follow mode for real-time streaming - Toggle timestamps on/off - Auto-scroll control - Copy logs to clipboard - Integration from app detail page
