---
type: map
title: >-
  Custom action execution checks per-app actions before blueprint actions and
  always gates on can_execute()
description: >-
  run_custom_action_handler looks up per-app custom actions first, falls back to
  blueprint actions, and only runs an action if CustomAction::can_execute()
  (approval status + expiration) passes.
tags:
  - custom-actions
  - authorization
  - blueprints
kk_schema_version: 3
kk_id: map-custom-action-execution-lookup-and-gate
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When a custom action is executed, the handler first looks for a matching action in the app's own `AppSettings.custom_actions`; only if none is found there does it fall back to the blueprint-defined action for that service. Per-app actions found this way are always required to pass `CustomAction::can_execute()`, which checks both approval status (must be Approved, not Pending/Rejected/Revoked) and TTL expiration, before the command is allowed to run — this is the single gate that enforces the approval workflow, independent of which permission (`action_read`/`action_write`) is required to reach the handler.
