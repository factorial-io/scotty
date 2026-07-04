---
type: map
title: Log streaming behavior for stopped vs missing containers
description: >-
  Stopped/exited containers return retained historical logs instead of an error;
  only a truly missing container is an error.
tags:
  - logs
  - docker
  - websocket
kk_schema_version: 3
kk_id: map-container-log-streaming-for-stopped-containers
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When a log stream is requested for a service whose container is not running (status Exited, Dead, Paused, Stopping, etc.), the system retrieves and streams the container's retained historical logs via `LogsStreamData` messages and ends with `LogsStreamEnded`, rather than rejecting the request. For a running container, historical logs are streamed followed by live output as usual.

A missing container (never created, or removed) is treated differently: the system still returns a `LogsStreamError` indicating the container could not be found. This distinguishes "container exists but stopped" (serve historical logs) from "no container at all" (error).
