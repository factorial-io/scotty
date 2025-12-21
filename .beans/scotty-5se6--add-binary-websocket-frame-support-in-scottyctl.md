---
# scotty-5se6
title: Add binary WebSocket frame support in scottyctl
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-54nc
---

# Description  Extend WebSocket client to send binary frames for shell input.  **Implementation** (scottyctl/src/websocket.rs): ```rust impl AuthenticatedWebSocket {     // NEW: Send binary shell input     pub async fn send_shell_input(         &mut self,          session_id: Uuid,          data: &[u8]     ) -> anyhow::Result<()> {         // Format: [session_id (16 bytes)] + [data]         let mut frame = Vec::with_capacity(16 + data.len());         frame.extend_from_slice(session_id.as_bytes());         frame.extend_from_slice(data);                  self.sender             .send(Message::Binary(frame))             .await             .context("Failed to send shell input")     } } ```  **Binary frame format**: ``` Byte:  0              16              n        ├──────────────┼───────────────┤        │ Session UUID │  Shell Data   │        │  (16 bytes)  │ (variable)    │        └──────────────┴───────────────┘ ```  **Testing**: - Unit test: Verify frame format (UUID + data) - Unit test: Test with various data sizes (1 byte, 8KB, 1MB)  **Time estimate**: 1-2 hours
