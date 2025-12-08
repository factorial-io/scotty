---
title: Detect and handle piped stdin in scottyctl shell command
status: open
priority: 1
issue_type: task
depends_on:
  scotty-f4e02: parent-child
created_at: 2025-12-08T23:52:50.210887+00:00
updated_at: 2025-12-08T23:52:50.210887+00:00
---

# Description

Add stdin detection and streaming for shell commands in command mode.

**Implementation**:
- Add has_piped_stdin() function using !std::io::stdin().is_terminal()
- Modify run_command_mode() to spawn stdin reader task when pipe detected
- Handle WebSocket sender sharing (use Arc<Mutex> or mpsc channel pattern)

**Key code changes** (scottyctl/src/commands/apps/shell.rs):
```rust
async fn run_command_mode(...) {
    if has_piped_stdin() {
        tokio::spawn(stream_stdin_to_websocket(ws_sender, session_id));
    }
    // existing output handling...
}

async fn stream_stdin_to_websocket(ws, session_id) {
    let mut stdin = tokio::io::stdin();
    let mut buffer = vec![0u8; 8192];
    while let Ok(n) = stdin.read(&mut buffer).await {
        if n == 0 { break; }
        ws.send_shell_input(session_id, &buffer[..n]).await?;
    }
}
```

**Testing**:
- echo test | scottyctl app:shell test-app svc -c "cat"
- cat file.txt | scottyctl app:shell test-app svc -c "wc -c"

**Time estimate**: 2-3 hours
