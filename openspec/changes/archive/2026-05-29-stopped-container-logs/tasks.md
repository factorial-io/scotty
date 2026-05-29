## 1. Server: log stream handler

- [x] 1.1 In `scotty/src/api/websocket/handlers/logs.rs`, remove the `if !container.is_running()` rejection branch (around line 84) that sends "is not running" error.
- [x] 1.2 Keep the genuine "container not found" path: when `app.find_container_by_service()` yields no container, still send `LogsStreamError`.
- [x] 1.3 Compute `effective_follow`: if the container is not running, force `follow = false`; otherwise use the client-requested value.
- [x] 1.4 When the client requested follow but the container is stopped, emit an informational message (reusing existing `LogsStreamData`/messaging) noting live follow is unavailable while the container is stopped.
- [x] 1.5 Pass `effective_follow` into `logs_service.start_stream(...)`.

## 2. Server: log streaming service

- [x] 2.1 In `scotty/src/docker/services/logs.rs`, confirm `start_stream`/`LogsOptions` use the passed `follow` value and that `follow = false` yields historical logs then a clean end (no infinite wait).
- [x] 2.2 Ensure the stream terminates with `LogsStreamEnded` after historical logs for non-follow requests.

## 3. CLI (scottyctl)

- [x] 3.1 In `scottyctl/src/commands/apps/logs.rs`, handle the follow-unavailable informational message gracefully (print notice, do not treat as error).
- [x] 3.2 Verify `app:logs` (and `app:logs -f`) against a stopped container prints historical logs and exits cleanly instead of erroring.

## 4. Frontend (UI)

- [x] 4.1 In `frontend/src/stores/containerLogsStore.ts`, stop treating a stopped-service log stream as an error state; render received `LogsStreamData`.
- [x] 4.2 In the service log view route (`frontend/src/routes/dashboard/[slug]/[service]/+page.svelte`), display historical logs for stopped services and show a "not live — container stopped" indicator.
- [x] 4.3 Run `bun run check` and `bun run lint` and fix any issues.

## 5. Tests & verification

- [~] 5.1 Add/adjust a server test covering: logs returned for a stopped container, and `LogsStreamError` for a missing container. (Partial: the missing-container error path is unchanged and exercised by existing code; the "logs returned for a stopped container" scenario is inherently a Docker integration test — no Docker mock harness exists in the repo, so it is covered by the manual e2e in 5.4.)
- [x] 5.2 Add a test asserting follow is downgraded (effective_follow = false) for a non-running container. (`resolve_follow_mode` unit tests in `scotty/src/api/websocket/handlers/logs.rs`.)
- [x] 5.3 Run `cargo test` for `scotty` and `scotty-core`; ensure green. (523 passed, 20 ignored.)
- [x] 5.4 Manual end-to-end check: stop a container in a test app, fetch logs via CLI and UI, confirm historical logs appear. (Confirmed working in both CLI and UI.)
