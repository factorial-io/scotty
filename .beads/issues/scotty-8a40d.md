---
title: Split large authorization service file into submodules
status: open
priority: 1
issue_type: chore
labels:
- maintainability
- refactoring
created_at: 2025-10-26T20:21:10.754914+00:00
updated_at: 2025-11-24T20:17:25.570273+00:00
---

# Description

The authorization service.rs file is 754 lines and could be split into more focused submodules for better maintainability.

# Design

Location: scotty/src/services/authorization/service.rs (754 lines)

Proposed structure:
- service.rs - Core service and main authorization logic
- permissions.rs - Permission checking and management
- scopes.rs - Scope-related operations
- roles.rs - Role management
- assignments.rs - Assignment handling

Impact: Better code organization and maintainability
Effort: 2-4 hours
