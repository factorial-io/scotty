---
# scotty-rc2w
title: Fix lost-update race in app settings mutations
status: todo
type: bug
priority: high
created_at: 2026-05-28T00:00:00Z
updated_at: 2026-05-28T00:00:00Z
parent: scotty-0ry2
---

# Description

The app-mutating REST handlers follow a non-atomic read-modify-write cycle against the in-memory app store (`SharedAppList`):

```rust
let app = state.apps.get_app(name).await;          // read + clone
let mut settings = app.settings.clone().unwrap_or_default();
settings.mutate(...);                               // modify clone
updated_app.save_settings().await?;                 // persist to disk
state.apps.update_app(updated_app).await?;          // write back
```

Two concurrent requests both read the same snapshot, mutate their own clones, and the last writer wins — silently dropping the other change. For custom actions this means, e.g., a concurrent `approve` + `revoke` on the same action can resurrect a revoked action, or two `create` calls can drop one action.

This is a pre-existing pattern shared by other mutations (notifications add/remove, custom action create/delete, and the admin approve/reject/revoke handlers), but PR #809 (per-app custom actions) widens the surface with five more mutating handlers, so it is worth fixing centrally.

# Affected locations

- `scotty/src/api/rest/handlers/apps/custom_action_management.rs` (create, delete)
- `scotty/src/api/rest/handlers/admin/custom_actions.rs` (`update_action_status`: approve/reject/revoke)
- `scotty/src/api/rest/handlers/apps/notify.rs` (add/remove) — same pattern
- Store: `scotty-core/src/apps/shared_app_list.rs`

# Design

Add an atomic mutation API to `SharedAppList` that holds the write lock across the whole read-modify-(persist)-write cycle, e.g.:

```rust
pub async fn update_app_with<F>(&self, app_name: &str, f: F) -> anyhow::Result<AppData>
where
    F: FnOnce(&mut AppData) -> anyhow::Result<()>;
```

The closure mutates the live entry while the lock is held; persistence (`save_settings`) happens inside the critical section so concurrent callers serialize on the same app. Migrate the custom-action and notification handlers to use it. Consider per-app locking (a keyed mutex map) if holding the global write lock across disk I/O proves too coarse.

# Acceptance criteria

- [ ] Atomic update helper added to `SharedAppList`
- [ ] Custom action create/delete/approve/reject/revoke handlers use it
- [ ] A concurrent-mutation test demonstrates no lost updates
