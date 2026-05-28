---
# scotty-54nc
title: 'Phase 1: Implement binary stdin/stdout piping for shell commands'
status: scrapped
type: feature
priority: critical
created_at: 2025-12-21T12:44:47Z
updated_at: 2026-05-26T14:54:46Z
parent: scotty-fad6
---

# Description  Add support for piping data to/from shell sessions in command mode.  Implementation approach: Use WebSocket binary frames (not base64-encoded JSON) for best performance.  **Scope**: - Detect when stdin is piped (using std::io::IsTerminal) - Read stdin in binary chunks (8KB) using tokio::io::AsyncReadExt - Send chunks as WebSocket binary frames with format: [session_id (16 bytes)][data] - Server parses binary frames and forwards to Docker exec stdin  **Files to modify**: 1. scottyctl/src/websocket.rs: Add send_shell_input(session_id, bytes) method 2. scottyctl/src/commands/apps/shell.rs: Add stream_stdin_to_websocket() function 3. scotty/src/api/websocket/client.rs: Handle Message::Binary case (currently ignored) 4. scotty/src/docker/services/shell.rs: Add get_session_info() for auth lookup  **Time estimate**: 8-10 hours  **User workflows enabled**: - Database import: cat dump.sql.gz | scottyctl app:shell db mysql -c "mysql mydb" - File copy: scottyctl app:shell app svc -c "cat /path/file" > local-file - Backup: scottyctl app:shell app svc -c "tar -czf - /data" > backup.tar.gz

## Reasons for Scrapping

Superseded by OpenSpec change `app-file-transfer` (commits on bookmark `feat/app-file-transfer`).

The use case this bean targeted (piping binary stdin into a service to drive database imports, file copies, backups) is fully covered by the new `scottyctl app:cp` command:

```bash
cat dump.sql.gz | scottyctl app:cp - myapp:db:/tmp/dump.sql.gz
scottyctl app:cp myapp:web:/var/log/app.log - | gzip > app.log.gz
```

We explicitly chose not to add a binary-stdin frame to the WebSocket shell endpoint. Rationale (recorded in `openspec/changes/app-file-transfer/design.md` D1): HTTP chunked + tar is the right transport for unidirectional bulk transfer; defining a custom binary framing protocol on top of WS would have been pure overhead. The shell endpoint stays focused on interactive sessions.
