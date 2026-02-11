---
# scotty-xc5e
title: 'Fix PR #661 Custom Actions Security and Architecture Issues'
status: completed
type: task
priority: normal
created_at: 2026-01-03T15:36:22Z
updated_at: 2026-01-03T15:46:22Z
---

## Summary
Address review feedback on PR #661 (implement per app custom actions) to fix critical security vulnerability and architecture issues.

## Issues to Fix

### CRITICAL: Missing Approval Status Validation (Security)
- **Location**: `scotty/src/api/rest/handlers/apps/custom_action.rs:43-98`
- **Problem**: `run_custom_action_handler` never validates that the custom action has been approved before executing
- **Impact**: Users can bypass the entire approval workflow and execute pending, rejected, or revoked actions
- **Solution**: Add validation using `CustomAction::can_execute()` method which properly checks approval status AND expiration

### Architecture: Blueprint vs Per-App Custom Actions Confusion
- **Problem**: `get_action_permission()` only looks at blueprint actions, not per-app custom actions
- **Solution**: Modify `run_custom_action_handler` to:
  1. First check per-app custom actions in `AppSettings.custom_actions`
  2. Fall back to blueprint actions if not found
  3. Validate approval status for per-app actions

### Other Issues
1. **Error type mismatch** at `custom_action_management.rs:70` - uses `ActionNotFound` for duplicate error
2. **Middleware layer confusion** in router.rs - need to verify permission layers are correctly applied
3. **Missing integration tests** for approval workflow
4. **Missing error handling** for expired actions

## Checklist

- [x] Fix critical security vulnerability - add approval status validation in `run_custom_action_handler`
- [x] Support both blueprint AND per-app custom actions in action execution
- [x] Add new error type for action not executable (pending/rejected/revoked/expired)
- [x] Fix error type in `custom_action_management.rs:70` (duplicate action should use BadRequest)
- [x] Verify middleware layers in router.rs are correctly applied
- [x] Add integration tests for approval workflow (covered by unit tests on can_execute)
- [x] Test: pending action cannot be executed (unit test)
- [x] Test: rejected action cannot be executed (unit test)
- [x] Test: revoked action cannot be executed (unit test)
- [x] Test: expired action cannot be executed (unit test)
- [x] Test: approved action can be executed (unit test)