---
title: Update authorization service methods to use domain resolution
status: closed
priority: 2
issue_type: task
labels:
- authorization
- enhancement
created_at: 2025-11-08T17:23:39.158486+00:00
updated_at: 2025-11-24T20:17:25.577185+00:00
closed_at: 2025-11-09T00:11:30.028292+00:00
---

# Description

Replace direct assignment lookups with resolve_user_assignments() in scotty/src/services/authorization/service.rs:
- check_global_permission()
- check_permission_in_scopes()
- get_user_scopes()
- list_user_assignments()

Replace pattern:
let all_assignments = [
    config.assignments.get(user)...,
    config.assignments.get("*")...,
].concat();

With:
let all_assignments = self.resolve_user_assignments(user, &config);

Part of domain-based authorization implementation (scotty-5840d)
