---
type: practice
title: Wire-format changes must be backward compatible in both directions
description: >-
  Preflight only gates major.minor, so a patch-level wire change silently breaks
  a newer scottyctl against an older server.
tags:
  - api
  - compatibility
  - scottyctl
  - gotcha
kk_schema_version: 3
kk_id: practice-wire-format-changes-must-be-backward-compatible-in-both-directions
kk_derived_from:
  - 'f2e204e5-c4ad-4433-b498-0707aeed9618:practice:0'
kk_relates_to:
  - map-scottyctl-cli-structure
  - map-app-create-file-content-is-a-base64-string-on-the-wire
  - practice-frontend-backend-tight-coupling
  - practice-release-process-automation
kk_depends_on: []
kk_confidence: high
---
The scottyctl/server wire format must stay compatible in both directions: a newer client talking to an older server, as well as the reverse. Accepting a new encoding server-side is only half the job.

`PreflightChecker::check_compatibility` (`scottyctl/src/preflight.rs`) calls `VersionManager::are_compatible`, which compares only major and minor. Any wire change shipped in a patch release therefore passes preflight unnoticed, and the failure surfaces at the endpoint instead — a serde shape mismatch on `POST /apps/create` becomes axum's stock **422 Unprocessable Entity**, with no hint that the cause is version skew.

When a payload shape has to change, either keep the old encoding readable by the old server, or gate the new encoding on the server version already fetched from `/api/v1/info` during preflight. Deploying servers before clients is the operational mitigation, but it is not enforced anywhere.

Note that this is the opposite of the frontend's contract: the frontend ships in lockstep with the backend and needs no compatibility, while scottyctl is installed independently and does.

<!-- kk:related:start -->
# Related

- Related: [map-scottyctl-cli-structure](/cli/map-scottyctl-cli-structure.md)
- Related: [map-app-create-file-content-is-a-base64-string-on-the-wire](/map-app-create-file-content-base64-wire-format.md)
- Related: [practice-frontend-backend-tight-coupling](/frontend/practice-frontend-backend-tight-coupling.md)
- Related: [practice-release-process-automation](/workflow/practice-release-process-automation.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [f2e204e5-c4ad-4433-b498-0707aeed9618:practice:0](f2e204e5-c4ad-4433-b498-0707aeed9618:practice:0)
<!-- kk:citations:end -->
