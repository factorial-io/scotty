---
type: practice
title: 'Frontend tooling uses bun, not npm'
description: >-
  Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must
  pass before push.
tags:
  - frontend
  - tooling
  - bun
kk_schema_version: 3
kk_id: practice-frontend-uses-bun
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
The frontend uses `bun`, not `npm`: `bun install`, `bun run dev`, `bun run build`, `bun run check` (type checking). `bun run lint` (Prettier + ESLint) must pass before push.
