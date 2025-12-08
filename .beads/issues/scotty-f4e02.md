---
title: 'Phase 1: Implement binary stdin/stdout piping for shell commands'
status: open
priority: 1
issue_type: task
depends_on:
  scotty-1836f: parent-child
created_at: 2025-12-08T23:52:38.237493+00:00
updated_at: 2025-12-08T23:52:38.237493+00:00
---

# Description

Add support for piping data to/from shell sessions in command mode.

Implementation approach: Use WebSocket binary frames (not base64-encoded JSON) for best performance.

**Scope**:
- Detect when stdin is piped (using std::io::IsTerminal)
- Read stdin in binary chunks (8KB) using tokio::io::AsyncReadExt
- Send chunks as WebSocket binary frames with format: [session_id (16 bytes)][data]
- Server parses binary frames and forwards to Docker exec stdin

**Files to modify**:
1. scottyctl/src/websocket.rs: Add send_shell_input(session_id, bytes) method
2. scottyctl/src/commands/apps/shell.rs: Add stream_stdin_to_websocket() function
3. scotty/src/api/websocket/client.rs: Handle Message::Binary case (currently ignored)
4. scotty/src/docker/services/shell.rs: Add get_session_info() for auth lookup

**Time estimate**: 8-10 hours

**User workflows enabled**:
- Database import: cat dump.sql.gz | scottyctl app:shell db mysql -c "mysql mydb"
- File copy: scottyctl app:shell app svc -c "cat /path/file" > local-file
- Backup: scottyctl app:shell app svc -c "tar -czf - /data" > backup.tar.gz
