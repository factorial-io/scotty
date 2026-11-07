# Proposal: align-specs-with-current-behavior

## Why

A code review (2026-11-07) found two specs that describe behavior the app does not have. The implemented behavior is the intended one, so the specs follow the code:

- `container-logs` says follow is refused for any non-running container and that the server sends a notice. The server only downgrades follow for terminal container states (`Exited`, `Dead`, `Removing`, `Empty`, see `scotty-core/src/apps/app_data/container.rs:112-120`). A paused, stopping, restarting or created container can resume, so live follow stays useful. There is no separate notice: the effective `follow` flag is echoed on `LogsStreamStarted` (`scotty/src/api/websocket/handlers/logs.rs:96-101`) and clients derive their message from it.
- `app-action-feedback` says a refused action's message is shown next to the action controls. The frontend shows it in the global error dialog (`frontend/src/stores/errorStore.ts:6-8`); a task that was accepted and later fails is shown as an error state with tooltip on the control (`frontend/src/components/start-stop-app-action.svelte`).

## What Changes

- `container-logs`: follow is downgraded only for terminal container states; the server echoes the effective `follow` flag instead of sending a notice; the UI's "not live" indication is driven by that flag.
- `app-action-feedback`: refused requests are reported in the global error dialog; failed tasks are reported on the action control.

No code changes.

## Capabilities

### Modified Capabilities

- `container-logs`
- `app-action-feedback`
