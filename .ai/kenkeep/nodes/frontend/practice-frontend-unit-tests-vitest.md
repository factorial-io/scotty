---
type: practice
title: Frontend unit tests run with Vitest, colocated as *.test.ts
description: >-
  `bun run test` runs Vitest through vite.config.ts (jsdom, src/**/*.test.ts);
  tests sit next to the module, mock $app/* and stores with vi.mock, stub
  browser APIs with vi.stubGlobal, and set the page URL via a
  @vitest-environment-options docblock. No component tests yet.
tags:
  - frontend
  - testing
  - vitest
  - workflow
kk_schema_version: 3
kk_id: practice-frontend-unit-tests-vitest
kk_derived_from: []
kk_relates_to:
  - map-frontend-src-layout
  - practice-frontend-uses-bun
kk_depends_on: []
kk_confidence: high
---
The frontend's unit tests run with Vitest, configured in the `test` block of `frontend/vite.config.ts` (which imports `defineConfig` from `vitest/config`) so `$lib` and `$app/*` resolve exactly as in the app. `bun run test` runs `svelte-kit sync && vitest run`; `bun run test:watch` watches. CI runs the tests in the frontend job between lint and build.

Conventions:
- Tests are colocated: `src/lib/foo.ts` has `src/lib/foo.test.ts`; `src/stores/bar.ts` has `src/stores/bar.test.ts`. Prettier and ESLint cover them like any other source file.
- Environment is jsdom, so `sessionStorage`, `window` and `Response` exist. `fetch` is stubbed per test with `vi.stubGlobal('fetch', vi.fn())` and cleaned up with `vi.unstubAllGlobals()`.
- `window.location` cannot be reassigned in jsdom; set the page URL per file with a first-line docblock `// @vitest-environment-options {"url": "https://host:port/path"}` (see `src/lib/ws.test.ts` and `ws-http.test.ts` for wss vs ws).
- SvelteKit modules are mocked with `vi.mock('$app/navigation', ...)` and `vi.mock('$app/paths', ...)`. Stores that import other stores are mocked the same way; a factory that needs `writable` must `await import('svelte/store')` inside the factory because `vi.mock` is hoisted above top-level variables.
- Timers in polling code (`monitorTask`) are driven with `vi.useFakeTimers()` and `vi.advanceTimersByTimeAsync`.
- `passWithNoTests: true` keeps `bun run test` green in a checkout without tests.

Not covered yet: Svelte component tests (would need `@testing-library/svelte`) and end-to-end tests.
