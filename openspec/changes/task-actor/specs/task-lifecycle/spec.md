## Purpose

Defines the lifecycle guarantees of app-operation tasks: every task started for an app operation reaches a terminal state exactly once, that state is set only by the operation owner, and observers see a consistent view of state and output.

## ADDED Requirements

### Requirement: Every task reaches a terminal state

The system SHALL move every app-operation task from `Running` to exactly one terminal state, `Finished` or `Failed`, regardless of how the underlying operation ends. A task MUST NOT remain `Running` after the operation has stopped executing.

#### Scenario: Operation completes normally
- **WHEN** all steps of an app operation succeed
- **THEN** the task is `Finished` with a finish time set

#### Scenario: A step reports an error
- **WHEN** a step of an app operation fails with an error
- **THEN** the task is `Failed` with a finish time set and a status message describing the failure

#### Scenario: A step panics
- **WHEN** a step of an app operation panics before the operation's completion step has run
- **THEN** the task is `Failed` with a finish time set and a status message stating that the operation aborted unexpectedly
- **AND** the panic is logged with the app name and task id

#### Scenario: The failure handling itself fails
- **WHEN** the operation's own failure-handling step returns an error before it could terminate the task
- **THEN** the task is still `Failed` with a finish time set

### Requirement: Terminal state is set once and only by the operation owner

Intermediate steps of an operation, such as individual subprocesses, SHALL NOT change the task state. Only the operation owner SHALL set the terminal state, and once set it SHALL NOT be overwritten.

#### Scenario: Subprocess exits during a multi-step operation
- **WHEN** one subprocess of a multi-step operation exits, successfully or not
- **THEN** the task remains `Running` and records that subprocess's exit code
- **AND** the operation decides from the exit code whether to continue or fail

#### Scenario: Subprocess cannot be started
- **WHEN** a subprocess cannot be spawned or waited on
- **THEN** the reason is recorded in the task output
- **AND** the operation treats the step as failed

#### Scenario: A second terminal event arrives
- **WHEN** the owner attempts to terminate a task that is already terminal
- **THEN** the task state, finish time and output are left unchanged
- **AND** the attempt is logged

#### Scenario: Nested operation
- **WHEN** an operation runs another operation as one of its steps (create runs rebuild, destroy runs purge)
- **THEN** only the outer operation terminates the task and sends its notification
- **AND** a failure in the inner operation fails the outer operation

### Requirement: Observers see a consistent task view

Any snapshot of a task exposed over the REST API or WebSocket SHALL be internally consistent: a terminal snapshot SHALL contain the final status line, and output lines SHALL be delivered in the order they were produced. A subscriber SHALL learn about a change in task state or output without polling.

#### Scenario: Poller reads a terminal task
- **WHEN** a client reads a task whose state is `Finished` or `Failed`
- **THEN** the returned output already contains the completion status line

#### Scenario: Output stream ends
- **WHEN** a client streams a task's output and the task terminates
- **THEN** the client receives every remaining output line before the stream-ended message

#### Scenario: Client waits for completion
- **WHEN** scottyctl waits for a task
- **THEN** it reports failure whenever the task state is `Failed`, regardless of the last subprocess exit code
- **AND** it shows a subprocess exit code only when that exit code is non-zero
