---
# scotty-cpk8
title: Frontend interactive shell UI with xterm.js
status: todo
type: feature
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:48Z
parent: scotty-m89i
---

# Description  Add interactive shell terminal to web frontend using xterm.js. Backend ShellService is complete with WebSocket support, but frontend has no shell UI.  # Design  - Integrate xterm.js for terminal emulation - WebSocket handlers for ShellSession* messages - TTY resize support on window resize - Copy/paste support - Terminal settings (font size, theme) - Shell session management UI (list, create, terminate) - Session timeout indicator  # Acceptance Criteria  - Can open interactive shell to any service from web UI - Terminal emulation works correctly (colors, escape sequences) - Copy/paste functional - Terminal resizes properly - Session list shows active shells - Clean session termination - Security: requires Shell permission
