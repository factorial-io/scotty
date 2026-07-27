---
type: map
title: Casbin policy assignment keys are formatted differently per auth_mode
description: >-
  OAuth mode keys assignments by email; bearer mode uses
  'identifier:token_name'; dev mode has no applicable assignments.
tags:
  - authorization
  - casbin
  - auth
kk_schema_version: 3
kk_id: map-authorization-assignment-key-format-by-auth-mode
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
In `config/casbin/policy.yaml`, the format of `assignments` keys depends on `api.auth_mode`: in OAuth mode, keys are email addresses extracted from OIDC token claims (e.g. `"alice@example.com"`); in bearer mode, keys use the `identifier:token_name` prefix mapping to `api.bearer_tokens.<token_name>` (e.g. `"identifier:deployment"`); in dev mode, the fixed dev user from `api.dev_user_*` is used and authorization assignments don't apply.

Apps declare their scope membership via a `scopes` list in their own `.scotty.yml`, and the authorization system maps apps to scopes from that source.
