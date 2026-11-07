# container-logs

## MODIFIED Requirements

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

### Requirement: UI displays logs for stopped services

The web UI SHALL display the historical logs of a stopped or exited service in the service log view, and SHALL indicate that the stream is not live whenever the server reports the effective follow setting as disabled, instead of showing only an error state.

#### Scenario: Viewing logs of an exited service in the UI

- **WHEN** a user opens the log view for a service whose container has exited
- **THEN** the UI shows the container's historical log output
- **AND** indicates that the log stream is not live because the container is stopped
