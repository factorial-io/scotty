---
# scotty-qaby
title: Fix WebSocket shell session authorization bypass
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T13:33:23Z
parent: scotty-uq4t
---

# Description  WebSocket shell session terminate and resize operations don't verify session ownership. Any authenticated user can terminate or resize ANY shell session if they know the session UUID.  Security Impact: Session hijacking vulnerability  Locations: - scotty/src/api/websocket/handlers/shell.rs:189-232 (terminate) - scotty/src/api/websocket/handlers/shell.rs:141-186 (resize)  Current Behavior: - User authentication is verified - But session ownership is NOT verified before operations - Any user can operate on any session with known UUID  Required Fix: - Store session ownership (user_id → session_id mapping) - Verify requesting user owns the session before allowing terminate/resize - Return proper authorization error if ownership check fails  References: PR #467 review from 2025-11-24
