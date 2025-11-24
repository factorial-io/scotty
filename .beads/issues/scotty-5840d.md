---
title: Implement domain-based authorization for role assignments
status: closed
priority: 2
issue_type: task
labels:
- authorization
- enhancement
- oauth
depends_on:
  scotty-01528: blocks
  scotty-2625e: blocks
  scotty-7a384: blocks
  scotty-32a16: blocks
  scotty-52f17: blocks
  scotty-da05a: blocks
  scotty-f7013: blocks
created_at: 2025-11-08T17:22:26.076452+00:00
updated_at: 2025-11-24T20:17:25.557157+00:00
closed_at: 2025-11-09T00:11:30.298035+00:00
---

# Description

Extend the authorization system to support domain-based role assignments (e.g., @factorial.io) in addition to exact email matches.

Design decisions:
- Precedence: 1) Exact email match 2) Domain match 3) Wildcard (additive)
- Syntax: Use @ prefix in assignments map (e.g., @factorial.io)
- Validation: Domain must have @ prefix, non-empty, contain dot, no additional @ chars
- Backward compatible: No API changes, existing configs work unchanged

Assignment resolution logic:
1. Check exact email match - if found, use it (skip domain lookup)
2. If no exact match, extract domain and check @domain pattern
3. Always add wildcard (*) assignments additively

Integration points:
- Add domain extraction and validation utilities
- Add resolve_user_assignments() helper method
- Update service methods: check_global_permission, check_permission_in_scopes, get_user_scopes, list_user_assignments
- Add validation in create_assignment_handler API endpoint

Testing:
- Unit tests for domain extraction, validation, resolution precedence
- Integration tests for complete OAuth flow with domain assignments
- Manual testing scenarios for CLI and API
