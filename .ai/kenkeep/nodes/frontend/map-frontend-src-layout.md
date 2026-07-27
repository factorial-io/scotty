---
type: map
title: Frontend src/ layout and dev-server proxy targets
description: >-
  Frontend src/ splits into routes, stores (webSocketStore.ts, userStore.ts),
  generated (ts-rs output), and lib; dev server proxies /api and /ws to the
  backend.
tags:
  - frontend
  - structure
  - sveltekit
kk_schema_version: 3
kk_id: map-frontend-src-layout
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Frontend source is organized under `frontend/src/`: `routes/` holds SvelteKit routes (dashboard, login, oauth callback, tasks); `stores/` holds Svelte stores, notably `webSocketStore.ts` for WebSocket connection management and `userStore.ts` for authentication state; `generated/` holds TypeScript types auto-generated from Rust via ts-rs; `lib/` holds shared components and utilities.

The SvelteKit dev server (`http://localhost:5173`) proxies `/api/*` to the backend REST API, `/ws/*` to backend WebSocket endpoints, and `/rapidoc` to the OpenAPI documentation, all pointing at the Scotty backend on `http://localhost:21342` by default (configurable in `vite.config.ts`).
