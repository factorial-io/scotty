---
title: Implement app:shell CLI command in scottyctl
status: closed
priority: 1
issue_type: feature
depends_on:
  scotty-541fa: parent-child
created_at: 2025-10-24T22:58:21.798713+00:00
updated_at: 2025-11-24T20:17:25.568831+00:00
closed_at: 2025-11-03T13:07:16.855054+00:00
---

# Description

Add the app:shell command to scottyctl CLI. Backend ShellService is fully implemented and tested, but the CLI command is missing. Need to create scottyctl/src/commands/apps/shell.rs with WebSocket-based terminal integration.

# Design

- Create ShellCommand struct in cli.rs
- Implement WebSocket-based shell handler in commands/apps/shell.rs
- Add TTY resize handling and raw terminal mode
- Support interactive input/output with proper escape sequences
- Reuse AuthenticatedWebSocket pattern from logs command

# Acceptance Criteria

- scottyctl app:shell myapp web opens interactive shell
- Terminal escape sequences work correctly
- Ctrl+C and Ctrl+D handled properly
- Session cleanup on disconnect
- Help text and examples documented
