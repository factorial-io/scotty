# container-logs Specification

## Purpose

Define how the system serves container log streams, including support for retrieving the historical logs of stopped or exited containers, the behavior of follow mode against non-running containers, error handling for missing containers, and the corresponding web UI presentation.

## Requirements

### Requirement: Logs for stopped containers

The system SHALL return the retained historical logs of a container whose service is not running (status `Exited`, `Dead`, `Paused`, `Stopping`, etc.) when a log stream is requested, instead of rejecting the request.

#### Scenario: Fetch logs of an exited container

- **WHEN** a user requests logs for a service whose container has exited
- **THEN** the system retrieves the container's historical logs from Docker
- **AND** streams them to the client as `LogsStreamData` messages
- **AND** ends the stream with `LogsStreamEnded` once all retained logs have been sent

#### Scenario: Fetch logs of a running container is unchanged

- **WHEN** a user requests logs for a service whose container is running
- **THEN** the system streams historical logs followed by live output exactly as before

### Requirement: Follow mode is unavailable for stopped containers

The system SHALL NOT follow the logs of a container in a terminal state (`Exited`, `Dead`, `Removing`, or no state). For such a container it SHALL return the historical logs, report follow as disabled on `LogsStreamStarted`, and end the stream cleanly. Containers in any other state, including `Paused`, `Stopping`, `Restarting` and `Created`, SHALL be followed live.

#### Scenario: Follow requested on a stopped container

- **WHEN** a user requests logs with follow enabled for a service whose container has exited or is dead
- **THEN** the system sends `LogsStreamStarted` with `follow` set to false
- **AND** returns the retained historical logs
- **AND** ends the stream with `LogsStreamEnded` instead of emitting `LogsStreamError`

#### Scenario: Follow requested on a paused container

- **WHEN** a user requests logs with follow enabled for a service whose container is paused
- **THEN** the system sends `LogsStreamStarted` with `follow` set to true
- **AND** streams historical logs and then live output until the client disconnects

#### Scenario: Follow requested on a running container

- **WHEN** a user requests logs with follow enabled for a service whose container is running
- **THEN** the system streams historical logs and then continues streaming live output until the client disconnects

#### Scenario: CLI reports the downgrade

- **WHEN** the CLI requests logs with `--follow` and receives `LogsStreamStarted` with `follow` set to false
- **THEN** it prints that live follow is unavailable because the container has stopped and that historical logs are shown

### Requirement: Missing container is still an error

The system SHALL continue to return a clear error when the requested app or service has no corresponding container at all (never created, removed), distinguishing this from a container that exists but is stopped.

#### Scenario: No container exists for the service

- **WHEN** a user requests logs for a service that has no container
- **THEN** the system sends a `LogsStreamError` indicating the container could not be found

### Requirement: UI displays logs for stopped services

The web UI SHALL display the historical logs of a stopped or exited service in the service log view, and SHALL indicate that the stream is not live whenever the server reports the effective follow setting as disabled, instead of showing only an error state.

#### Scenario: Viewing logs of an exited service in the UI

- **WHEN** a user opens the log view for a service whose container has exited
- **THEN** the UI shows the container's historical log output
- **AND** indicates that the log stream is not live because the container is stopped
