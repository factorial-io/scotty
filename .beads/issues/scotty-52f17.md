---
title: Implement assignment resolution logic
status: closed
priority: 2
issue_type: task
labels:
- authorization
- enhancement
created_at: 2025-11-08T17:23:33.670195+00:00
updated_at: 2025-11-24T20:17:25.581142+00:00
closed_at: 2025-11-09T00:11:29.979267+00:00
---

# Description

Add resolve_user_assignments() method to AuthorizationService in scotty/src/services/authorization/service.rs.

Logic:
1. Check exact email in assignments map
2. If not found, extract domain and check @domain key
3. Always add wildcard (*) assignments

Return combined Vec<Assignment>.

Part of domain-based authorization implementation (scotty-5840d)
