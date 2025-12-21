---
# scotty-7tcc
title: Add validation to assignment creation API
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:47Z
---

# Description  Update create_assignment_handler in scotty/src/api/rest/handlers/admin/assignments.rs.  Call validate_domain_assignment() on request.user_id before creating the assignment. Return appropriate error response if validation fails.  Part of domain-based authorization implementation (scotty-5840d)
