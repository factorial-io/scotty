---
type: practice
title: 'Frontend has no API versioning, evolves in lockstep with backend'
description: >-
  Scotty frontend is tightly coupled to the backend API; no versioning or
  backwards compatibility is maintained, so breaking API changes are acceptable.
tags:
  - frontend
  - api
  - architecture
kk_schema_version: 3
kk_id: practice-frontend-backend-tight-coupling
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The Scotty frontend is deliberately tightly coupled with the backend REST/WebSocket API. There is no API versioning and no backwards-compatibility guarantee between frontend and backend — they are meant to evolve together. Breaking API changes are therefore acceptable and expected, rather than something to avoid or gate behind a version negotiation.
