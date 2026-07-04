---
type: map
title: State machine handler errors always mark the task Failed
description: >-
  App lifecycle state machines propagate handler errors (and panics) up through
  spawn() so the owning task is always marked Failed instead of hanging forever.
tags:
  - state-machine
  - tasks
  - docker
kk_schema_version: 3
kk_id: map-state-machine-errors-bubble-to-task-failure
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The state machine runner's `spawn()` returns a joinable result that propagates handler errors instead of only logging them internally. Callers (including nested state machines used by create/destroy app flows) route through a single shared completion helper (`Context::complete_task()`) so a task is always marked Failed on handler error or panic, and the WebSocket completion broadcast always fires exactly once.

Any new state-machine handler or nested state machine must let errors propagate through this path rather than swallowing them, otherwise the owning task can be left running indefinitely with no terminal state reported to clients.
