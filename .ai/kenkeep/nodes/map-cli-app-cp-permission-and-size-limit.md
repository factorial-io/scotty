---
type: map
title: 'app:cp permission split and transfer size limit'
description: >-
  app:cp downloads need view permission, uploads need manage; transfers capped
  by SCOTTY__FILES__MAX_TRANSFER_SIZE (default 1GiB).
tags:
  - cli
  - app-cp
  - permissions
  - authorization
kk_schema_version: 3
kk_id: map-cli-app-cp-permission-and-size-limit
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`app:cp` streams files between a workstation and a service container. Unlike most app commands that use a single permission, transfers split by direction: downloads (container to local/stdout) require the `view` permission, while uploads (local/stdin to container) require `manage`.

Transfers are bounded by the server-side setting `SCOTTY__FILES__MAX_TRANSFER_SIZE`, which defaults to 1 GiB.
