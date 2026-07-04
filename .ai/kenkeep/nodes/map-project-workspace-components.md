---
type: map
title: Scotty workspace components
description: The crates/apps making up the Scotty micro-PaaS and what each one does.
tags:
  - architecture
  - workspace
  - overview
kk_schema_version: 3
kk_id: map-project-workspace-components
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty is a micro Platform-as-a-Service for managing Docker Compose-based applications, split into these workspace members:

- **scotty**: HTTP server (REST API + WebSocket) for managing Docker Compose apps.
- **scottyctl**: CLI client for the scotty server.
- **scotty-core**: Shared business logic (Docker operations, settings, tasks).
- **scotty-types**: Shared type definitions (TypeScript-compatible via ts-rs).
- **frontend**: SvelteKit web interface, tightly coupled with the API with no backwards-compatibility requirement.
- **ts-generator**: Utility for generating TypeScript bindings from Rust types.
