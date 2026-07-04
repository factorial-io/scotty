---
type: map
title: Apps declare authorization scopes in .scotty.yml
description: >-
  App-to-scope membership is set via a `scopes:` list in .scotty.yml; unset apps
  land in `default`.
tags:
  - authorization
  - scopes
  - config
kk_schema_version: 3
kk_id: map-app-scope-declaration-in-scotty-yml
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Apps declare their scope membership for the authorization system in their own `.scotty.yml` file via a `scopes:` list (e.g. `scopes: ["frontend", "staging"]`), and can belong to multiple scopes at once. Apps without any explicit `scopes` entry are assigned to the `default` scope automatically.
