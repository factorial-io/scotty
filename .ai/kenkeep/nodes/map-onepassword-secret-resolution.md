---
type: map
title: '1Password secrets are resolved via op:// URIs in app env vars only'
description: >-
  Scotty resolves op://<connect-instance>/<vault-uuid>/<item-uuid>/<field> in
  env vars passed via app:create, not inside compose.yml.
tags:
  - 1password
  - secrets
  - configuration
kk_schema_version: 3
kk_id: map-onepassword-secret-resolution
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty can integrate with 1Password Connect, resolving a URI scheme `op://<connect-instance>/<vault-uuid>/<item-uuid>/<field>` for environment variables before running an action on an app (e.g. via `scottyctl app:create ... --env "DATABASE_PASSWORD=op://..."`).

Each connect instance needs a JWT configured under `onepassword.<instance>.jwt`/`.server`, normally injected via `SCOTTY__ONEPASSWORD__<INSTANCE>__JWT` env vars. Important limitation: Scotty does not resolve `op://` secrets referenced from environment variables inside `compose.yml` files — only those supplied through Scotty's own env-var mechanism.
