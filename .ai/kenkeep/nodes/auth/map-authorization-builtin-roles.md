---
type: map
title: Built-in authorization roles and their permission sets
description: >-
  Named RBAC roles beyond admin/developer/viewer: operator, system_admin,
  action_approver.
tags:
  - authorization
  - rbac
  - roles
  - casbin
kk_schema_version: 3
kk_id: map-authorization-builtin-roles
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Beyond the basic `admin` (`*`), `developer`, and `viewer` roles, Scotty's Casbin RBAC also defines: `operator` — operations without shell access (`[view, manage, logs, action_read]`); `system_admin` — authorization management only (`[admin_read, admin_write]`), deliberately separate from app management; and `action_approver` — can approve/reject/revoke pending custom actions (`[view, action_approve]`).

The separation between `admin` (manages apps) and `system_admin` (manages authorization config: scopes/roles/assignments) lets an installation grant authorization management without app access, or vice versa, implementing least-privilege for admin roles.
