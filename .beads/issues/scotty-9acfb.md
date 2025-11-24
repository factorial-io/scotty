---
title: Frontend interactive shell UI with xterm.js
status: open
priority: 2
issue_type: feature
depends_on:
  scotty-541fa: parent-child
  scotty-047bf: related
created_at: 2025-10-24T22:58:22.023108+00:00
updated_at: 2025-11-24T20:17:25.562043+00:00
---

# Description

Add interactive shell terminal to web frontend using xterm.js. Backend ShellService is complete with WebSocket support, but frontend has no shell UI.

# Design

- Integrate xterm.js for terminal emulation
- WebSocket handlers for ShellSession* messages
- TTY resize support on window resize
- Copy/paste support
- Terminal settings (font size, theme)
- Shell session management UI (list, create, terminate)
- Session timeout indicator

# Acceptance Criteria

- Can open interactive shell to any service from web UI
- Terminal emulation works correctly (colors, escape sequences)
- Copy/paste functional
- Terminal resizes properly
- Session list shows active shells
- Clean session termination
- Security: requires Shell permission
