---
# scotty-r7yt
title: Split large authorization service file into submodules
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T13:33:54Z
parent: scotty-f8ot
---

# Description  The authorization service.rs file is 754 lines and could be split into more focused submodules for better maintainability.  # Design  Location: scotty/src/services/authorization/service.rs (754 lines)  Proposed structure: - service.rs - Core service and main authorization logic - permissions.rs - Permission checking and management - scopes.rs - Scope-related operations - roles.rs - Role management - assignments.rs - Assignment handling  Impact: Better code organization and maintainability Effort: 2-4 hours
