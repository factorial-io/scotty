---
type: practice
title: Regenerate frontend TypeScript types after backend Rust type changes
description: >-
  After changing Rust types, run `cargo run --bin ts-generator` from the repo
  root to refresh generated TypeScript in frontend/src/generated/.
tags:
  - frontend
  - types
  - ts-rs
  - workflow
kk_schema_version: 3
kk_id: practice-frontend-types-regenerate-after-backend-change
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Frontend TypeScript types are generated from Rust types via `ts-rs`, not written by hand. Whenever backend Rust types change, the project convention is to run `cargo run --bin ts-generator` from the repository root, which updates all files under `frontend/src/generated/` (including `index.ts`). Skipping this step leaves the frontend's generated types out of sync with the backend.
