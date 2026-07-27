## ADDED Requirements

### Requirement: Container running state derived from Docker state and exit code

Scotty SHALL determine each container's running state from the Docker container `State.Status`, and SHALL additionally capture the container `State.ExitCode` so that a container which has stopped can be classified as either *completed successfully* or *failed*.

A container SHALL be considered **running** when its status is `Running`, `Created`, or `Restarting`.
A container SHALL be considered **completed** when its status is `Exited` and its exit code is `0`.
A container SHALL be considered **failed** when its status is `Exited` with a non-zero exit code, or `Dead`.

#### Scenario: Running web container

- **WHEN** a container reports Docker status `Running`
- **THEN** scotty classifies it as running

#### Scenario: One-shot init container that finished

- **WHEN** a container reports Docker status `Exited` with exit code `0`
- **THEN** scotty classifies it as completed (not failed and not running)

#### Scenario: Crashed container

- **WHEN** a container reports Docker status `Exited` with a non-zero exit code
- **THEN** scotty classifies it as failed

### Requirement: App status tolerates completed one-shot containers

Scotty SHALL compute app-level status so that completed one-shot/init containers do not prevent the app from reaching `Running`.

The app SHALL be `Running` when at least one container is running and every container is either running or completed (exit code 0).
The app SHALL be `Starting` when some but not all containers are running and none of the not-yet-running containers have failed.
The app SHALL be `Stopped` when no container is running and none has completed.

A failed container SHALL NOT by itself force the whole app out of `Running`; per-service status continues to reflect the failure for that service.

#### Scenario: App with running web service and completed init container

- **WHEN** an app has a web container in `Running` and an init container `Exited` with code `0`
- **THEN** the app status is `Running`

#### Scenario: App still starting

- **WHEN** an app has one container `Running` and another still `Created` (not yet running, not exited)
- **THEN** the app status is `Starting`

#### Scenario: Fully stopped app

- **WHEN** all of an app's containers are `Exited` and none completed with a running sibling
- **THEN** the app status is `Stopped`

### Requirement: Service URL visibility follows per-service running state

The system SHALL expose a service's URL when that specific service's container is running, independent of the overall app status.

The web UI SHALL render a clickable URL for each domain of a service whose container is running, and SHALL render the service as a disabled/non-clickable element when its container is not running.

#### Scenario: Running service in a not-fully-running app

- **WHEN** an app is not `Running` (e.g. `Starting`) but one of its services has a container in `Running` with configured domains
- **THEN** that service's URL is shown as a clickable link

#### Scenario: Stopped service

- **WHEN** a service's container is not running
- **THEN** that service's URL is not clickable

#### Scenario: Per-service status drives the button

- **WHEN** the frontend renders a service URL button
- **THEN** visibility is decided from that service's own status, not the app-level status
