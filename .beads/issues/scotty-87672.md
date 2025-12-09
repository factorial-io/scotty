---
title: Handle binary WebSocket frames in scotty server
status: open
priority: 1
issue_type: task
depends_on:
  scotty-f4e02: parent-child
created_at: 2025-12-08T23:53:16.682757+00:00
updated_at: 2025-12-08T23:53:16.682757+00:00
---

# Description

Process binary WebSocket frames and forward to shell sessions.

**Implementation** (scotty/src/api/websocket/client.rs):
```rust
while let Some(Ok(msg)) = receiver.next().await {
    match msg {
        Message::Text(text) => {
            // existing JSON handling (unchanged)
        }
        Message::Binary(bin) => {
            // NEW: Handle binary shell input
            handle_binary_shell_input(&state, client_id, &bin).await;
        }
    }
}

async fn handle_binary_shell_input(
    state: &SharedAppState,
    client_id: Uuid,
    data: &[u8],
) {
    // Parse: [session_id (16 bytes)] + [data]
    let session_id = Uuid::from_slice(&data[0..16])?;
    let shell_data = &data[16..];
    
    // Get session info for auth check
    let session = shell_service.get_session_info(session_id).await?;
    
    // Authorization check
    check_permission(user, &session.app_name, Permission::Shell).await?;
    
    // Forward to shell service
    let input = String::from_utf8_lossy(shell_data).to_string();
    shell_service.send_input(session_id, input).await?;
}
```

**Security**:
- Validate session_id exists
- Check Permission::Shell authorization
- Size limit per frame: 1MB (prevent DoS)

**Testing**:
- Integration test: Send binary frame, verify forwarded to container
- Test authorization denied case
- Test malformed frame handling

**Time estimate**: 3-4 hours
