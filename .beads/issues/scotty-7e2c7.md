---
title: Prevent concurrent tasks for the same app
status: open
priority: 2
issue_type: bug
labels:
- tasks
- ux
created_at: 2025-10-27T10:43:29.205844+00:00
updated_at: 2025-11-24T20:17:25.576198+00:00
---

# Description

Currently you can run multiple tasks for the same app at the same time, this should be prevented. On the other hand we need the possibility to cancel a running task to enable the UI again. The concurrent task prevention should be handled ideally by the backend.

# Design

The backend should track running tasks per app and reject new task requests if a task is already running for that app.

Implementation considerations:
1. TaskManager should track which app each task belongs to
2. Add endpoint to check if app has running tasks
3. Add task cancellation endpoint
4. Frontend UI should:
   - Disable action buttons when task is running
   - Show cancel button for running tasks
   - Re-enable UI when task completes/cancelled

Location: scotty/src/tasks/manager.rs
