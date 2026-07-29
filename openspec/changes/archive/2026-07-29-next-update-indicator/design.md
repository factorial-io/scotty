# Design: next-update-indicator

## Context

App state is refreshed by a single `clokwerk` job:

```
scheduler.running_app_check ("15m", config/default.yaml:16)
        │
setup.rs:15  schedule_app_check(...)   ← once at startup
setup.rs:42  .every(interval).run(schedule_app_check)
        │
find_apps() → inspect_app()  → app_data.last_checked = Local::now()   (find_apps.rs:146)
        │
SharedAppList → GET /apps/info/{id} → SecureJson(app_data)  (run.rs:111)
        │
appsStore.updateAppInfo() → dashboard/[slug]/+page.svelte
```

Two facts shape the design:

1. **The sweep is global, not per-app.** One `clokwerk` job inspects every app, so "next update" is one server-wide timestamp, not per-app state.
2. **`last_checked` is "last inspected", not "last swept".** `inspect_app()` is also reached from `update_app_data_handler.rs:31` and `task_completion_handler.rs:86`, so a manual rebuild bumps it without moving the sweep:

```
sweep          manual rebuild bumps        real next sweep
  │               last_checked                   │
──●───────────────────●────────────────────────  ●─────────
  t0                 t0+30s                    t0+15m
                       └── last_checked+15m ────────┘ t0+15m30s  ✗
```

The detail page already re-reads app data on store updates (`+page.svelte`, `apps.subscribe(...)` → `data = result`), so a field stamped on `AppData` stays reasonably fresh without new wiring.

## Goals / Non-Goals

**Goals:**
- Show a truthful "next update in …" on the app detail page.
- Keep the derivation server-side, so the frontend needs no knowledge of `scheduler.running_app_check`.
- One shared relative-time component, not a second near-duplicate.

**Non-Goals:**
- No new endpoint and no change to `/api/v1/info` (its response is fetched once per session and would go stale).
- No per-app scheduling — the sweep stays global.
- No removal of `last_checked` (metrics depend on it).
- No countdown ticking faster than the existing shared `time` store.

## Decisions

1. **Carry the value on the app payload, not on `/api/v1/info`.** The detail page already refetches app data on websocket-driven store updates, while `/api/v1/info` is fetched once in `sessionStore.ts:203` and cached for the session — a next-update time served there would freeze. Riding a global value on a per-app response is slightly odd, but it is the only path that is fresh without new plumbing.

2. **Store the next-due time in `AppState`; the sweep is the single writer.** `schedule_app_check` sets `next = now + running_app_check` at the *start* of each sweep (and at the startup call, `setup.rs:15`), so the value is available before the first apps are stamped. Sweep duration therefore shows up as a few seconds of pessimism, which is invisible against a 15-minute interval.

3. **Represent it as an `AtomicI64` unix-seconds field (`0` = unset), not a lock.** `AppState` is shared behind `Arc` and derives `Clone`/`Debug`; an atomic keeps interior mutability without introducing lock discipline, poisoning questions, or an async lock in a sync read path. Converted to `DateTime<Local>` at the read site.

4. **`inspect_app()` stamps `next_check` from that global value.** Alternative considered: compute `now + interval` inside `inspect_app`. Rejected — that reintroduces exactly the drift this change exists to remove, because `inspect_app` also runs for manual actions. Reading the global instead means a rebuild-triggered inspection reports the same next sweep the scheduler will actually run.

   Consequence: the field is populated uniformly for both the list and info endpoints, with no handler changes.

5. **Make `time-ago.svelte` sign-aware rather than adding a `TimeUntil` sibling.** Drop the `Math.max(0, …)` clamp; render `in <n> <unit>` for a negative elapsed time and keep `<n> <unit> ago` otherwise. It is the single component all four relative-time call sites route through, so one branch there beats a near-copy — and the other call sites only ever pass past timestamps, so their output is unchanged.

6. **An overdue timestamp renders as "ago", uncorrected.** If the server is paused or a sweep runs long, the pill reads `Next update 2 minutes ago`. Awkward, but self-explanatory (the sweep is overdue), self-correcting on the next sweep, and cheaper than a page-local "due now" branch that would need its own reactive clock. Revisit only if it shows up in practice.

7. **Null `next_check` hides the pill.** An app created but not yet swept (`create_app.rs:266`) has nothing truthful to show; a fabricated estimate would be worse than silence.

## Risks / Trade-offs

- [Adding a field to `AppData` touches every struct literal] → mechanical; the compiler enumerates them, and the proposal lists the known sites.
- [Global value on a per-app payload invites future confusion] → doc comment on the field stating it is server-global, mirroring the sweep.
- [`AppData` is a ts-rs type; frontend must be regenerated] → covered by a task; the frontend has no API versioning to keep compatible (`practice-frontend-backend-tight-coupling`).
- [Users lose the staleness reading they had] → `last_checked` remains in the API and the connectivity/status pills continue to convey liveness; if staleness turns out to be wanted alongside, it can go in the pill's `title` later.

## Migration Plan

Additive field plus a frontend swap; ships with the next build. No config or persisted state changes. Rollback = revert.

## Open Questions

- Should the pill also surface `last_checked` on hover, or is the forward-looking value alone sufficient? (Current plan: forward-looking only.)
