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

The system SHALL NOT attempt to follow (tail in real time) the logs of a non-running container. When follow is requested for a stopped container, the system SHALL return the available historical logs and inform the client that follow is not possible while the container is stopped, rather than failing with an error.

#### Scenario: Follow requested on a stopped container

- **WHEN** a user requests logs with follow enabled for a service whose container is stopped
- **THEN** the system returns the retained historical logs
- **AND** sends an informational notice that live follow is unavailable while the container is stopped
- **AND** ends the stream cleanly instead of emitting `LogsStreamError`

#### Scenario: Follow requested on a running container

- **WHEN** a user requests logs with follow enabled for a service whose container is running
- **THEN** the system streams historical logs and then continues streaming live output until the client disconnects

### Requirement: Missing container is still an error

The system SHALL continue to return a clear error when the requested app or service has no corresponding container at all (never created, removed), distinguishing this from a container that exists but is stopped.

#### Scenario: No container exists for the service

- **WHEN** a user requests logs for a service that has no container
- **THEN** the system sends a `LogsStreamError` indicating the container could not be found

### Requirement: UI displays logs for stopped services

The web UI SHALL display the historical logs of a stopped or exited service in the service log view, and SHALL indicate when live follow is unavailable because the container is not running, instead of showing only an error state.

#### Scenario: Viewing logs of an exited service in the UI

- **WHEN** a user opens the log view for a service whose container has exited
- **THEN** the UI shows the container's historical log output
- **AND** indicates that the log stream is not live because the container is stopped
