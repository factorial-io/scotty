# Operations, locking and progress

Scotty runs at most one operation that changes an app at a time. This page
describes which operations are exclusive, what a client sees when it asks for a
second one, and how the progress of a running operation is reported.

## One operation per app

Every operation that changes an app takes the app's operation lock first. The
following operations are exclusive. The name in brackets is how the operation
shows up in the `operation` field of the app data:

* create (`create`), run (`run`), stop (`stop`), rebuild (`rebuild`), purge
  (`purge`) and destroy (`destroy`)
* custom actions that require the `action_write` permission (`action:<name>`)
* the stop and destroy done by the TTL checker (`ttl-stop`, `ttl-destroy`)
* adopting an unsupported app (`adopt`)
* edits of the app's settings: creating, deleting, approving, rejecting and
  revoking custom actions, and adding or removing notifications (`settings`)

Apps are identified by their directory name. Operations on different apps do
not block each other.

These are **not** exclusive and never wait for or get refused because of a
running operation:

* custom actions that require only `action_read`
* log streams, shell sessions, file downloads and file uploads
  (`scottyctl app:cp`)
* listing apps, app info and task queries

An upload that runs during a rebuild, purge or destroy can fail if its target
container is being recreated or removed.

The scheduled app sweep and the proxy-network reconciler do not take the lock
either.

The lock lives in memory. It is released when the operation ends, whether it
succeeds, fails or panics, and a server restart leaves no app locked. For
operations that run as a task, it is released just before the task is marked as
finished or failed, so a client that waits for the task can start the next
operation immediately.

### The 409 response

A request for an exclusive operation on a busy app is refused with
`409 Conflict`. No task is created and the running operation is not affected.
Permissions are checked first: a caller without the permission gets `403`, not
`409`.

```json
{
  "error": true,
  "message": "App shop is busy: rebuild (task 6f1c2a9e-3b47-4c59-8d0e-5a7b1e2c9f10) running since 2026-11-12T10:03:11Z; try again when it has finished",
  "operation": {
    "operation": "rebuild",
    "task_id": "6f1c2a9e-3b47-4c59-8d0e-5a7b1e2c9f10",
    "started_at": "2026-11-12T10:03:11Z"
  }
}
```

`operation.task_id` is `null` for operations without a task (`adopt`,
`settings`) and for the short moment before the task exists.

### The operation on app data

App data (`apps/list`, `apps/info/{app}`, the app data returned when an action
starts, and the MCP tools `list_apps` and `get_app`) has an `operation` field.
It is `null` while no operation runs, otherwise:

```json
"operation": {
  "operation": "rebuild",
  "task_id": "6f1c2a9e-3b47-4c59-8d0e-5a7b1e2c9f10",
  "started_at": "2026-11-12T10:03:11Z"
}
```

The value comes from the lock itself, so the scheduled sweep does not change
it. `action_read` actions never show up here. Seeing `operation` does not give
access to the task: tasks keep following the rules in
[Authorization](authorization.md).

When an operation starts or ends, the server sends an `AppInfoUpdated` message
with the app name over the WebSocket to every authenticated client that may
view the app. When an app is created or destroyed it also sends
`AppListUpdated`. The web UI uses this to show the running operation, disable
the controls that would change the app and update without a reload, wherever
the operation was started.

### TTL checker

The TTL checker never stops or destroys an app that has an operation running.
It skips the app and looks at it again on its next run (see `scheduler.ttl_check`
in [Configuration](configuration.md)).

### Maximum operation duration

An operation that runs longer than `apps.max_operation_duration` is aborted:
its running subprocess is killed, the task fails with a message naming the
limit, a warning is logged and the lock is released. The default is `1h`. See
[App settings](configuration.md#app-settings).

```yaml
apps:
  max_operation_duration: "2h"
```

or `SCOTTY__APPS__MAX_OPERATION_DURATION=2h`.

### scottyctl waits

By default `scottyctl` waits when the app is busy. It prints which operation
(and task) it waits for, polls the app until its `operation` is `null`, then
sends the command again. If another client takes the app first, it keeps
waiting. This lets overlapping CI runs on one app queue up instead of failing.

```shell
scottyctl app:rebuild shop
# Waiting for rebuild (task 6f1c2a9e-...) on shop...
```

With the global flag `--no-wait` scottyctl fails at once instead, prints the
server's message and exits with a non-zero code:

```shell
scottyctl --no-wait app:stop shop
```

Raw API clients and older versions of scottyctl get the `409` and have to retry
themselves.

## Task progress

The details of a task (`GET /api/v1/authenticated/task/{id}`) have a `progress` field for operations run as tasks:

```json
"progress": {
  "fraction": 0.42,
  "status": "Pulling images",
  "steps": [
    { "name": "Log into registry", "state": "done" },
    { "name": "Prepare network", "state": "done" },
    { "name": "Pull images", "state": "running" },
    { "name": "Build images", "state": "pending" },
    { "name": "Stop containers", "state": "pending" },
    { "name": "Start containers", "state": "pending" },
    { "name": "Wait for containers", "state": "pending" },
    { "name": "Run post actions", "state": "pending" }
  ],
  "transfers": [
    {
      "image": "nginx:1.27",
      "phase": "downloading",
      "current_bytes": 18350080,
      "total_bytes": 52428800,
      "layers_done": 5,
      "layers_total": 7
    }
  ]
}
```

* `fraction` is the progress of the whole operation, from 0 to 1. While the
  task runs it never decreases, even when a pull discovers more data than it
  knew before. It is 1 when the task has finished and keeps its last value when
  the task failed.
* `status` is one line saying what happens now.
* `steps` lists every step of the operation, in order, from the start. `state`
  is one of `pending`, `running`, `done`, `failed` or `skipped`. Custom actions
  list one step per command they run.
* `transfers` has one entry per image while `docker compose pull` runs, or
  while `docker compose up` pulls images that are missing locally, and is
  empty otherwise. `phase` is `waiting`, `downloading`, `extracting` or `done`.
  `total_bytes` and `layers_total` cover the layers known so far and grow as
  more layers start. Layers that already exist locally count as done. The task
  output keeps readable lines such as the name of each pulled image, not the raw
  progress events of Docker.
* Starting containers reports progress too: the step follows the images `up`
  pulls and the containers it has started out of the app's services, and the
  output gets one line per started container.

Changes are pushed to the clients that may see the task as `TaskInfoUpdated`
WebSocket messages, at most four times per second per task, and always when a
step changes and when the task ends.

The MCP tool `get_task` returns `progress` too, with `fraction`, `status` and
the steps as `<name>: <state>` lines; it leaves out `transfers`, since the
status line already names the image and its byte counts during a pull.

### How clients show it

* The web UI shows a progress bar and the status line for the running operation
  in the app list and on the app page. The app page also lists the steps. The
  landing page of a stopped app shows the progress of the start instead of the
  raw output, and shows the output only when the start fails. When you may not
  see the task, only the running operation is shown.
* `scottyctl` shows the percentage and the status line in its animated status
  line while it waits for a task. Nothing is shown when the output is not a
  terminal.
