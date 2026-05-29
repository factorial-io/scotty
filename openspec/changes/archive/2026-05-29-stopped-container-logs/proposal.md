## Why

Users frequently need to inspect the logs of a container *after* it has crashed or exited — that is exactly when the logs matter most for debugging. Today scotty rejects any log request for a container that is not running, even though Docker retains and can return the historical logs of stopped containers. This forces users to debug failures blind, both from the CLI (`app:logs`) and the web UI.

## What Changes

- Allow fetching logs from stopped containers (status `Exited`, `Dead`, `Paused`, etc.), not just running ones.
- Remove the hard `is_running()` rejection in the log WebSocket handler; replace it with state-aware behavior.
- For non-running containers, return the retained historical logs and end the stream cleanly (no live follow, since there is nothing to follow). For running containers, behavior is unchanged (historical + optional live follow via `--follow`).
- When `--follow` is requested on a stopped container, return the available historical logs and inform the user that follow is not possible while the container is stopped, instead of failing.
- UI: surface logs for stopped/exited services in the service log view rather than showing only an error.

## Capabilities

### New Capabilities
- `container-logs`: Streaming and retrieval of container logs for an app's services, covering both running containers (historical + live follow) and stopped containers (historical only).

### Modified Capabilities
<!-- None: no prior spec exists for this behavior. -->

## Impact

- **Server**: `scotty/src/api/websocket/handlers/logs.rs` (remove/replace the `is_running()` gate), `scotty/src/docker/services/logs.rs` (force non-follow mode for stopped containers).
- **CLI**: `scottyctl/src/commands/apps/logs.rs` — handle the "follow not available for stopped container" notice gracefully.
- **Frontend**: `frontend/src/stores/containerLogsStore.ts`, service log view route — render logs for stopped services and convey follow-not-available state.
- **No API/protocol breaking changes**: existing WebSocket messages (`StartLogStream`, `LogsStreamData`, `LogsStreamEnded`, `LogsStreamError`) are reused.
- **No new dependencies**: relies on existing Bollard `logs()` support for stopped containers.
