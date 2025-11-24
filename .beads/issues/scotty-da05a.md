---
title: Add validation to assignment creation API
status: closed
priority: 2
issue_type: task
labels:
- api
- authorization
- enhancement
created_at: 2025-11-08T17:23:43.818652+00:00
updated_at: 2025-11-24T20:17:25.571495+00:00
closed_at: 2025-11-09T00:11:30.076496+00:00
---

# Description

Update create_assignment_handler in scotty/src/api/rest/handlers/admin/assignments.rs.

Call validate_domain_assignment() on request.user_id before creating the assignment. Return appropriate error response if validation fails.

Part of domain-based authorization implementation (scotty-5840d)
