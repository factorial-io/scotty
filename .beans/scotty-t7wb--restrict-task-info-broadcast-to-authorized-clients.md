---
# scotty-t7wb
title: Restrict task visibility to authorized clients
status: todo
type: bug
priority: high
created_at: 2026-11-07T09:47:57Z
updated_at: 2026-11-07T09:47:57Z
parent: scotty-uq4t
---

# Description  `TaskActor` sends `TaskInfoUpdated` with `broadcast_to_all` (scotty/src/tasks/actor.rs), which reaches every connected WebSocket client, including unauthenticated connections and users without access to the task's app or scope. This discloses task IDs, app names and task status across tenants. Related: `StartTaskOutputStream` only checks authentication, not permissions, for tasks without an `app_name` (scotty/src/api/websocket/handlers/tasks.rs).  The REST endpoints have the same gap: `GET /api/v1/authenticated/tasks` lists every task and `GET /api/v1/authenticated/task/{uuid}` returns any task to any authenticated user (recorded as a known gap in scotty/tests/fixtures/authorization_matrix.yaml).  Careful: during `app:create` the app is not registered in a scope until creation finishes, and scottyctl polls the task meanwhile, so a plain `view` filter would hide the creator's own create task. Options: record the creating user on the task, or register the app's scopes before the task starts.  # Fix  Send task updates only to authenticated clients with `view` permission on the task's app; decide who may see tasks without an app (admins only?). Filter the REST task list and details the same way, then tighten the matrix entries. Add tests.  Found during the code survey for openspec change `security-review-and-pentest-plan`.
