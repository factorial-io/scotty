---
title: Enhance auth:status to show user's assigned permissions
status: closed
priority: 3
issue_type: feature
created_at: 2025-11-25T18:08:14.156700+00:00
updated_at: 2025-11-25T18:18:23.727629+00:00
closed_at: 2025-11-25T18:18:23.727629+00:00
---

# Description

The `scottyctl auth:status` command should display all permissions the authenticated user has been assigned.

**Current behavior:**
- Shows basic authentication status (logged in, user info)

**Desired behavior:**
- Show all role assignments for the user
- Show scopes the user has access to
- Show effective permissions (view, manage, create, destroy, shell, logs, admin:*)

**Example output:**
```
Authenticated as: user@example.com

Role Assignments:
  - admin on scopes: [*]
  - developer on scopes: [client-a, qa]

Effective Permissions:
  - view: [*]
  - manage: [client-a, qa]
  - create: [client-a, qa]
  - destroy: [client-a]
  - shell: [client-a, qa]
  - logs: [*]
```

**Implementation notes:**
- May need new API endpoint to return user's effective permissions
- Or extend existing auth status endpoint response
