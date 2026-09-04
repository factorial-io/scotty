# task-lifecycle Specification

## Purpose
Defines the lifecycle guarantees of app-operation tasks: every task started for an app operation eventually reaches a terminal state, and that state truthfully reflects whether the operation completed.

## Requirements

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

Intermediate steps of an operation, such as individual subprocesses, SHALL NOT change the task state. Only the operation's owner SHALL set the terminal state, and once set it SHALL NOT be overwritten by the supervision that guards against a missed termination.

#### Scenario: Subprocess exits during a multi-step operation
- **WHEN** one subprocess of a multi-step operation exits successfully
- **THEN** the task remains `Running` until the operation's completion step runs

#### Scenario: Supervision observes an already terminated task
- **WHEN** the operation ends with an error after its completion step already set `Failed`
- **THEN** the task state, finish time and status output are left unchanged
