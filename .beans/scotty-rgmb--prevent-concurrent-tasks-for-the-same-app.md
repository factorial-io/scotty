---
# scotty-rgmb
title: Prevent concurrent tasks for the same app
status: todo
type: bug
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  Currently you can run multiple tasks for the same app at the same time, this should be prevented. On the other hand we need the possibility to cancel a running task to enable the UI again. The concurrent task prevention should be handled ideally by the backend.  # Design  The backend should track running tasks per app and reject new task requests if a task is already running for that app.  Implementation considerations: 1. TaskManager should track which app each task belongs to 2. Add endpoint to check if app has running tasks 3. Add task cancellation endpoint 4. Frontend UI should:    - Disable action buttons when task is running    - Show cancel button for running tasks    - Re-enable UI when task completes/cancelled  Location: scotty/src/tasks/manager.rs
