---
type: map
title: 'Scotty''s scope: single-node micro-PaaS, not a cluster orchestrator'
description: >-
  Scotty is a single-node docker-compose orchestrator for ephemeral review apps,
  not a Kubernetes/Nomad replacement.
tags:
  - architecture
  - scope
  - positioning
kk_schema_version: 3
kk_id: map-scotty-product-scope
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty is a micro-Platform-as-a-Service for managing docker-compose-based apps via a simple UI, CLI, and REST API, primarily to host ephemeral review apps. It is a single-node solution: it does not orchestrate apps across a cluster of machines and has no support for scaling apps, so it is not a substitute for Nomad, Kubernetes, or OpenShift when fine-grained execution control or clustering is needed.

Access control is optional scope-based authorization plus basic auth to block unauthorized access and discourage robot indexing of apps.

While not a full replacement for tools like Dockyard or Portainer, Scotty does provide basic debugging capabilities: real-time log viewing via both the CLI and web UI, and interactive shell access to containers via the CLI.
