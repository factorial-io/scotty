---
# scotty-x54b
title: Show next app-state update instead of last update on app detail
status: completed
type: feature
priority: normal
created_at: 2026-07-29T21:02:19Z
updated_at: 2026-07-29T21:13:28Z
---

Implements openspec change next-update-indicator: server tracks next scheduled app-check sweep in AppState, stamps next_check on AppData in inspect_app, frontend pill shows 'Next update in N minutes' via a sign-aware time-ago component.

## Summary of Changes

Server now tracks the next scheduled app-check sweep in `AppState.next_app_check` (an `AtomicI64` of unix seconds, written at the start of each sweep in `schedule_app_check`). `inspect_app` stamps it onto `AppData.next_check` by *reading* that global rather than computing `now + interval` -- which is what keeps a mid-interval manual run/stop/rebuild from shifting the reported next check.

Frontend: `time-ago.svelte` delegates to a new sign-aware `lib/relative-time.ts` (`in 12 minutes` / `12 minutes ago`), and the app detail pill reads `Next update in N minutes`, hidden when `next_check` is null.

Verified live with a 1m interval: after a manual `app:run`, `last_checked` moved 23:12:00 -> 23:12:25 while `next_check` stayed at 23:13:00 (naive extrapolation would have said 23:13:25).

OpenSpec change: `next-update-indicator` (13/13 tasks).
