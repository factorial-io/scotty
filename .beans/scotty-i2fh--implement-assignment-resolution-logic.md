---
# scotty-i2fh
title: Implement assignment resolution logic
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Add resolve_user_assignments() method to AuthorizationService in scotty/src/services/authorization/service.rs.  Logic: 1. Check exact email in assignments map 2. If not found, extract domain and check @domain key 3. Always add wildcard (*) assignments  Return combined Vec<Assignment>.  Part of domain-based authorization implementation (scotty-5840d)
