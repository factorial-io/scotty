---
# scotty-70mc
title: Scoped token gets 403 on apps/info for freshly created app (#894)
status: completed
type: bug
priority: normal
created_at: 2026-09-04T20:47:56Z
updated_at: 2026-09-04T20:48:15Z
---

GitHub issue factorial-io/scotty#894. Root cause: TaskManager::handle_task_completion marked the shared task Finished after every subprocess (compose pull/build/up), so clients saw Finished before UpdateAppData synced the app's scopes to Casbin.

- [x] Reproduce with Docker-backed integration test (scotty/tests/test_scoped_create_visibility.rs, #[ignore])
- [x] Fix: successful subprocess exit no longer marks task Finished; only TaskCompletionHandler does
- [x] Move task-finished metric to Context::complete_task
- [x] Unit tests in scotty/src/tasks/manager.rs

## Summary of Changes

- handle_task_completion in scotty/src/tasks/manager.rs now only records last_exit_code on success and leaves the task Running; a non-zero exit still fails the task. set_task_finished removed.
- Task-finished metric moved to Context::complete_task, the single place a task ends.
- New ignored Docker integration test scotty/tests/test_scoped_create_visibility.rs (fails 3/3 before the fix, passes after).
- Two unit tests in manager.rs run in CI.
- Note: the issue's suggested fix (call set_app_scopes in UpdateAppDataHandler) was not needed; inspect_app already syncs scopes. The bug was the premature Finished state.
