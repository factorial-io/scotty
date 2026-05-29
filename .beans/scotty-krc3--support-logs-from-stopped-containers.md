---
# scotty-krc3
title: Support logs from stopped containers
status: completed
type: feature
priority: normal
created_at: 2026-05-29T20:12:39Z
updated_at: 2026-05-29T21:26:32Z
---

Allow fetching container logs for stopped/exited containers in both scottyctl and the UI. Currently only works for running containers due to an is_running() gate in the log WebSocket handler. OpenSpec change: stopped-container-logs.

## Summary of Changes

Implemented OpenSpec change `stopped-container-logs`. Logs can now be fetched from stopped/exited containers in both CLI and UI.

**Server** (`scotty/src/api/websocket/handlers/logs.rs`): removed the `is_running()` rejection gate; added `resolve_follow_mode(requested_follow, is_running)` helper that downgrades follow to a one-shot historical fetch for non-running containers and flags the downgrade. The handler builds a notice and passes `effective_follow` + notice to the log service. Genuine 'container not found' still errors.

**Server** (`scotty/src/docker/services/logs.rs`): `start_stream` gained a `notice: Option<String>` param; when set, emits a synthetic stderr `LogsStreamData` line after `LogsStreamStarted`. Non-follow streams already end cleanly via the idle timeout ('All logs delivered').

**CLI** (`scottyctl/src/commands/apps/logs.rs`): tracks effective follow from `LogsStreamStarted.follow`; uses it for end-of-stream messaging and the stop decision so a downgraded follow exits cleanly.

**Frontend** (`+page.svelte`): added a 'container not running — historical logs, follow unavailable' info banner keyed off service status. The logs store already rendered `LogsStreamData` and only errors on `LogsStreamError`, so no store change was needed.

**Tests**: 3 unit tests for `resolve_follow_mode`. Full suite green (523 passed). Clippy clean. Frontend check + lint pass.

## Remaining
- Manual e2e (CLI + UI against a stopped container) not run — requires a live Docker/server environment.
- A Docker-integration test for 'logs returned for stopped container' was not added (no Docker mock harness in repo).

## Fix: UI showed no logs (duplicate Svelte key)

Root cause: `unified-output.svelte` renders logs with a keyed each `{#each lines as line (line.sequence)}`. The server-injected notice line used `sequence: 0`, colliding with the first historical log line (converter also starts at 0). Svelte throws on duplicate keys, breaking the whole list — UI rendered nothing. CLI was unaffected (ignores sequence).

Resolution: removed the synthetic server-side notice line entirely. The follow-downgrade is now signalled only via `LogsStreamStarted.follow=false`: CLI prints a yellow notice when its requested follow differs from the effective follow; UI shows the existing 'not running' banner. Removed the `notice` param from `start_stream`. Tests green (523), clippy clean.

## Verified
Confirmed working end-to-end in both CLI and UI by the user. 5.1 remains a partial (no Docker mock harness for an automated 'logs from stopped container' integration test; covered by manual e2e).

## Post-review refinements (rust-engineer)

- **Terminal-only follow downgrade** (was: any non-running). Added `ContainerState::is_terminal()` (Exited/Dead/Removing/Empty) in scotty-core; handler now downgrades follow only for terminal containers, so Paused/Stopping/Created keep live follow. `resolve_follow_mode` param renamed `is_running` -> `can_follow_live`. Added 2 unit tests for `is_terminal`.
- **Frontend banner** now keyed off terminal states (was non-running) and reworded; Paused/Stopping no longer mislabeled as follow-unavailable.
- **CLI**: removed the redundant/misleading post-loop 'No historical logs found. Try using --follow' message (the in-loop LogsStreamEnded handler already reports no-logs); reworded downgrade notice to 'has stopped'.

Full suite 525 passed, clippy clean, frontend check+lint clean.

## Addressed PR #817 review feedback
- Simplified `resolve_follow_mode` to return `bool` (removed the dead `_follow_downgraded` tuple element); downgrade is detected client-side via `LogsStreamStarted.follow`.
- Documented what `ContainerStatus::Empty` means in `is_terminal()` and why it's treated as terminal.
- Added a sync-note comment on the frontend `isTerminal` list pointing to `is_terminal()` as the authoritative source.
- Trimmed redundant comments in the `resolve_follow_mode` unit tests; linked the spec from the helper doc.
525 tests pass, clippy clean, frontend check+lint clean.
