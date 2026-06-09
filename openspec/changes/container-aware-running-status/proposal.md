## Why

Scotty marks an app as `Running` only when **every** container is in the `Running` state, and the web UI only shows a service's URL when the whole app is `Running`. Apps that use init/one-shot containers (which run once and `Exit 0`) therefore get stuck at `Starting` forever, and the URLs of their genuinely-running web services are hidden — even though those services are up and reachable. A running container should expose its URL regardless of whether sibling one-shot containers have already exited.

## What Changes

- Read the container exit code from Docker (`State.ExitCode`) during inspection and carry it on `ContainerState`, so a clean one-shot exit (`Exited` + code 0) can be distinguished from a crash (non-zero exit / `Dead`).
- Treat a container that has `Exited` with code 0 as **completed** rather than as a failure when computing app-level status, so a finished init container no longer holds the app at `Starting`.
- Redefine app-level status aggregation: an app is `Running` when all of its long-running containers are `Running` and any completed one-shot containers exited successfully; `Starting` while containers are still coming up; `Stopped` when nothing is running and nothing has completed.
- Gate URL visibility on the **individual service's** running state instead of the app-level status, so a running service shows its clickable URL even when the overall app is not fully `Running`. **BREAKING** (UI behavior): service URL buttons are now driven by per-service status.
- Expose per-service running state to the frontend so `app-service-button` can decide visibility from `service.status` rather than the app status passed in by callers.

## Capabilities

### New Capabilities
- `app-running-status`: How scotty derives container-level and app-level running status from Docker container state (including exit codes and one-shot/init containers), and how that status governs whether a service exposes its URL.

### Modified Capabilities
<!-- None: no existing spec defines app/service status behavior. -->

## Impact

- **scotty-core**: `apps/app_data/container.rs` (`ContainerState`, `ContainerStatus`, `is_running`), `apps/app_data/status.rs` (`get_app_status_from_services`), `apps/app_data/data.rs` (status assignment, `urls()`).
- **scotty**: `docker/find_apps.rs::inspect_docker_container` (read `State.ExitCode`).
- **scotty-types / generated TS bindings**: `ContainerState` gains an exit-code/completion field.
- **frontend**: `components/app-service-button.svelte` and its callers in `routes/dashboard/**` switch URL gating to per-service status; `types.ts` `AppService` gains the new field.
- **Tests**: `scotty-core` status aggregation unit tests; any integration test asserting app status for partially-exited apps.
