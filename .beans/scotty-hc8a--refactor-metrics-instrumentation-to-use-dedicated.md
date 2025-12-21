---
# scotty-hc8a
title: Refactor metrics instrumentation to use dedicated helper functions
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Review and refactor existing metrics code in log streaming and other services to move metrics recording into dedicated helper functions, keeping business logic clean and separated from instrumentation code.  # Design  Pattern established in scotty-14b43: - Create dedicated helper functions like `record_task_added_metrics()` and `record_task_finished_metrics()` - Move all `if let Some(m) = metrics::get_metrics()` blocks into these helpers - Keep business logic methods focused on their primary responsibility  Files to review and refactor: - scotty/src/docker/services/logs.rs (log streaming metrics) - scotty/src/docker/services/shell.rs (shell session metrics - if implemented) - Any other services with inline metrics code  Benefits: - Cleaner separation of concerns - Easier to test business logic without metrics - Consistent metrics instrumentation pattern - Better code readability
