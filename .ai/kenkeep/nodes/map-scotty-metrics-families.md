---
type: map
title: Scotty metrics families and prefix
description: >-
  All Scotty metrics use the scotty_ prefix, grouped by subsystem: log
  streaming, shell sessions, websocket, tasks, HTTP server, memory, application
  fleet, and Tokio runtime.
tags:
  - observability
  - metrics
kk_schema_version: 3
kk_id: map-scotty-metrics-families
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: low
---
Scotty instruments metrics across its major subsystems, all under the `scotty.`/`scotty_` prefix: log streaming (active/total streams, duration, lines, errors), shell sessions (active/total, duration, errors, timeouts), WebSocket (active connections, messages sent/received, auth failures), task output streaming (active/total tasks, duration, failures, output lines), HTTP server (active/total requests, request duration, with method/path/status attributes), process memory (RSS, virtual bytes), application fleet (total apps, apps by status, services per app, last-check age), and the Tokio async runtime (worker count, task lifecycle, poll counts/duration, slow polls, idle/scheduling timings).
