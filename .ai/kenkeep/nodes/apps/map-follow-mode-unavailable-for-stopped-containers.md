---
type: map
title: 'Follow mode is a no-op notice, not an error, on stopped containers'
description: >-
  Requesting live log follow on a stopped container returns historical logs plus
  an informational notice and a clean stream end, not LogsStreamError.
tags:
  - logs
  - docker
  - websocket
  - frontend
kk_schema_version: 3
kk_id: map-follow-mode-unavailable-for-stopped-containers
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The system does not attempt to tail logs in real time for a non-running container. If follow is requested for a stopped container, it returns the available historical logs, sends an informational notice that live follow is unavailable while the container is stopped, and ends the stream cleanly (no `LogsStreamError`). For a running container, follow continues streaming live output until the client disconnects.

The web UI mirrors this: the service log view displays historical output for a stopped/exited service and indicates that the stream is not live because the container is stopped, instead of showing only an error state.
