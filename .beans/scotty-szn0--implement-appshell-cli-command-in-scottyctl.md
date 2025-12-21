---
# scotty-szn0
title: Implement app:shell CLI command in scottyctl
status: completed
type: feature
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-rsgr
---

# Description  Add the app:shell command to scottyctl CLI. Backend ShellService is fully implemented and tested, but the CLI command is missing. Need to create scottyctl/src/commands/apps/shell.rs with WebSocket-based terminal integration.  # Design  - Create ShellCommand struct in cli.rs - Implement WebSocket-based shell handler in commands/apps/shell.rs - Add TTY resize handling and raw terminal mode - Support interactive input/output with proper escape sequences - Reuse AuthenticatedWebSocket pattern from logs command  # Acceptance Criteria  - scottyctl app:shell myapp web opens interactive shell - Terminal escape sequences work correctly - Ctrl+C and Ctrl+D handled properly - Session cleanup on disconnect - Help text and examples documented
