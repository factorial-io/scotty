---
type: map
title: Scotty Docker image ships binaries and non-sensitive config only
description: >-
  The official Docker image bundles binaries, Casbin model, and blueprints, but
  no secrets — those are supplied at runtime.
tags:
  - docker
  - deployment
  - config
  - security
kk_schema_version: 3
kk_id: map-docker-image-excludes-secrets
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The Docker image includes only the binaries and non-sensitive configuration files (Casbin model, blueprints). Configuration with secrets must be provided at runtime, either by mounting a config directory read-only or by passing environment variables (e.g. `SCOTTY__API__BEARER_TOKENS__ADMIN`, `SCOTTY__APPS__DOMAIN_SUFFIX`).
