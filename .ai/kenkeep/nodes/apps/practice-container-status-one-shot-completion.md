---
type: practice
title: >-
  Running status treats clean one-shot exits as completed, gates URLs
  per-service
description: >-
  App status aggregation distinguishes a clean Exited(0) one-shot container from
  a crash, and the frontend shows a service's URL based on that service's own
  status rather than the aggregate app status.
tags:
  - docker
  - status
  - container
  - frontend
kk_schema_version: 3
kk_id: practice-container-status-one-shot-completion
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty classifies a container as completed (as opposed to failed) when its Docker state is Exited with exit code 0, distinct from a crash (non-zero exit or Dead). App-level status aggregation treats an app as Running once every long-running container is Running and any one-shot/init containers among its services have completed cleanly; the app is Stopped only when nothing is running and nothing has completed, and Starting otherwise (including when a container has failed). This lets apps with init/setup containers reach Running instead of staying at Starting once their one-shot work finishes successfully.

Independent of the app-level status, the frontend gates a service's URL visibility on that individual service's own status rather than on the aggregate app status, so a genuinely running service exposes its clickable URL even while sibling containers are still starting or have already exited.
