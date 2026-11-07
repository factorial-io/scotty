---
# scotty-p3qn
title: Send WebSocket parse errors only to the sending client
status: todo
type: bug
priority: normal
created_at: 2026-11-07T09:47:57Z
updated_at: 2026-11-07T09:47:57Z
parent: scotty-uq4t
---

# Description  When a WebSocket message cannot be parsed, scotty/src/api/websocket/client.rs sends the error with `broadcast_to_all`, so every connected client receives it. Any client, including unauthenticated ones, can make the server spam all other clients.  # Fix  Use `send_error(client_id, ...)` instead and add a test.  Found during the code survey for openspec change `security-review-and-pentest-plan`.
