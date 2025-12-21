---
# scotty-jfgm
title: Update authorization service methods to use domain resolution
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:47Z
---

# Description  Replace direct assignment lookups with resolve_user_assignments() in scotty/src/services/authorization/service.rs: - check_global_permission() - check_permission_in_scopes() - get_user_scopes() - list_user_assignments()  Replace pattern: let all_assignments = [     config.assignments.get(user)...,     config.assignments.get("*")..., ].concat();  With: let all_assignments = self.resolve_user_assignments(user, &config);  Part of domain-based authorization implementation (scotty-5840d)
