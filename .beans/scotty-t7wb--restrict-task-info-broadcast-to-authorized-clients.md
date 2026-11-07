---
# scotty-t7wb
title: Restrict TaskInfoUpdated broadcast to authorized clients
status: todo
type: bug
priority: high
created_at: 2026-11-07T09:47:57Z
updated_at: 2026-11-07T09:47:57Z
parent: scotty-uq4t
---

# Description  `TaskActor` sends `TaskInfoUpdated` with `broadcast_to_all` (scotty/src/tasks/actor.rs), which reaches every connected WebSocket client, including unauthenticated connections and users without access to the task's app or scope. This discloses task IDs, app names and task status across tenants. Related: `StartTaskOutputStream` only checks authentication, not permissions, for tasks without an `app_name` (scotty/src/api/websocket/handlers/tasks.rs).  # Fix  Send task updates only to authenticated clients with `view` permission on the task's app; decide who may see tasks without an app (admins only?). Add tests.  Found during the code survey for openspec change `security-review-and-pentest-plan`.
