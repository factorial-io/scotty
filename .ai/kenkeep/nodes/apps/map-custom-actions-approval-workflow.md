---
type: map
title: Custom actions require approval before execution
description: >-
  Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved
  actions can run, gated by 4 dedicated permissions.
tags:
  - custom-actions
  - workflow
  - security
kk_schema_version: 3
kk_id: map-custom-actions-approval-workflow
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Custom actions let users define and execute arbitrary commands on app services, gated by an approval workflow for security control. Four dedicated permissions: `action_read` (execute read-only actions), `action_write` (execute state-modifying actions), `action_manage` (create/list/delete actions for apps in the user's scope), `action_approve` (approve/reject pending actions, admin-level).

Status workflow: a new action starts **Pending**, can be **approve**d to **Approved** or **reject**ed to **Rejected**; an Approved action can be **revoke**d back to **Revoked**; actions can also **Expire** if a TTL is configured. Only **Approved** actions can be executed.

Data model: `scotty-core/src/settings/custom_action.rs`. API handlers: `scotty/src/api/rest/handlers/apps/custom_action*.rs`. CLI: `scottyctl/src/commands/apps/actions.rs`, `scottyctl/src/commands/admin.rs`.
