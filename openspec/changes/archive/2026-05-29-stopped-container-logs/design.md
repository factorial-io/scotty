## Context

Container logs are streamed over WebSocket. The CLI (`app:logs`) and the web UI both send a `StartLogStream` request; the server resolves the service to a container and streams `LogsStreamData` until `LogsStreamEnded`.

Current state and constraints:
- The WebSocket handler `scotty/src/api/websocket/handlers/logs.rs` rejects the request with an error when `!container.is_running()` (around line 84), before any Docker call. `is_running()` (`scotty-core/src/apps/app_data/container.rs`) is true only for `Running`/`Created`/`Restarting`.
- The actual log fetch in `scotty/src/docker/services/logs.rs` uses Bollard `docker.logs(&container_id, LogsOptions { follow, tail, timestamps, stdout, stderr, .. })`. Bollard `logs()` already returns retained historical logs for stopped containers; only `follow: true` is meaningless for them (it returns the historical logs then completes).
- Container enumeration (`scotty/src/docker/find_apps.rs`) already discovers stopped containers via `docker-compose ps -a` and records their real status — so the container object exists for stopped services.

The blocker is purely the application-layer gate. Removing it and making `follow` state-aware is sufficient.

## Goals / Non-Goals

**Goals:**
- Return retained logs for stopped containers from both CLI and UI.
- Keep running-container behavior (historical + live follow) unchanged.
- Treat "container exists but stopped" differently from "no container at all".
- Avoid hanging a follow stream on a container that will never produce more output.

**Non-Goals:**
- No persistence of logs beyond what Docker itself retains (no scotty-side log archival).
- No change to the WebSocket message protocol / no new message variants.
- No automatic restart or re-creation of stopped containers to read logs.

## Decisions

**1. Replace the `is_running()` rejection with a state-aware branch.**
In `handle_start_log_stream`, instead of returning an error when the container is not running, keep going. Determine an `effective_follow` value: if the container is not running, force `follow = false` regardless of the client's request. Only the genuine "container not found" case (service has no container) remains an error path.

- *Alternative considered*: leave the handler untouched and only relax `is_running()`. Rejected — `is_running()` is used elsewhere and changing its semantics is riskier than a local branch in the logs handler.

**2. Force non-follow mode for stopped containers in the log service.**
Pass the computed `effective_follow` into `logs_service.start_stream()`. With `follow = false`, Bollard `logs()` returns the historical buffer and the stream completes naturally, producing a clean `LogsStreamEnded`. No timeout hacks needed.

- *Alternative considered*: pass `follow: true` and rely on Docker completing the stream. Rejected — relying on stream-completion semantics is less predictable; explicitly disabling follow is clearer and matches intent.

**3. Inform, don't fail, when follow is requested on a stopped container.**
When the client requested follow but the container is stopped, emit an informational `LogsStreamData` line (or a dedicated info message reusing existing variants) noting that live follow is unavailable, then stream historical logs and end. The CLI prints this notice; the UI shows a "not live — container stopped" indicator.

- *Alternative considered*: silently downgrade to non-follow. Rejected — users explicitly asking to follow deserve feedback about why output stopped.

**4. UI surfaces stopped-state logs.**
`containerLogsStore.ts` and the service log route already render `LogsStreamData`. The change is to not treat a stopped service as an error state and to display the follow-unavailable indicator. Status is already available in app/container data the UI receives.

## Risks / Trade-offs

- [Docker may have already pruned logs for a long-dead/removed container] → For a stopped-but-present container Docker still returns its retained logs; if the container was removed entirely there is no container object and we correctly return "not found". Acceptable.
- [Informational notice reuses existing message variants] → Keeps protocol stable but the notice is plain text; the UI distinguishes it via container status rather than a typed field. Acceptable for now; a typed field can be added later if needed.
- [Behavioral change for clients that depended on the old error] → Low risk; the old error ("not running") was a dead-end with no useful workflow built on it.

## Migration Plan

- Pure behavioral change, server + clients deployed together (frontend is embedded; CLI checks version compatibility). No data migration.
- Rollback: revert the handler/service/UI changes; protocol is unchanged so mixed versions degrade to the prior "not running" error at worst.

## Open Questions

- Should the follow-unavailable notice be a new typed WebSocket field instead of an inline text line? Deferred — start with inline text + status-driven UI; revisit if the UI needs richer signaling.
