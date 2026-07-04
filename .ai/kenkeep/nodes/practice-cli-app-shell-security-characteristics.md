---
type: practice
title: 'app:shell security characteristics to account for'
description: >-
  Shell sessions run as the container's default user, aren't logged by Scotty,
  and bypass app-level auth.
tags:
  - cli
  - app-shell
  - security
kk_schema_version: 3
kk_id: practice-cli-app-shell-security-characteristics
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
`app:shell` opens a direct interactive session (or single-command exec via `--command`) inside a running container, and requires the `shell` permission. Several security characteristics are worth keeping in mind when granting this permission: shell sessions run as the container's default user (often root), commands executed via shell are not logged by Scotty (container-level audit logging is needed if that matters), and shell access bypasses application-level authentication entirely.

Shell sessions are isolated to the specific container — users cannot reach other containers or the host directly, and network access follows the container's own network configuration.
