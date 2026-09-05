---
# scotty-gs3k
title: Add Vitest unit tests to the frontend (frontend-unit-tests)
status: completed
type: task
priority: normal
created_at: 2026-09-05T12:17:50Z
updated_at: 2026-09-05T12:21:49Z
---

Implement OpenSpec change frontend-unit-tests: Vitest runner via vite.config.ts, jsdom, first tests for lib/ and stores/, CI step.

## Summary of Changes

- Vitest 5 + jsdom as dev dependencies; `test`/`test:watch` scripts; `test` block in vite.config.ts (jsdom, src/**/*.test.ts, passWithNoTests).
- 21 tests in 7 files: relative-time, landingResume, ws (wss/ws via environment-options docblock), authenticatedApiCall (mocked fetch/session), tasksStore (fake timers), permissionStore (mocked userStore + API).
- CI frontend job runs `bun run test` between lint and build.
- kenkeep node practice-frontend-unit-tests-vitest.
