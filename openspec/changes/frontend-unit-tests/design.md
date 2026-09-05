## Context

See proposal.md. Frontend stack: SvelteKit 2, Svelte 5, Vite 8, TypeScript 6, bun as package manager, ESLint + Prettier gating pushes, `bun run check` (svelte-check) and `bun run build` in CI. State lives in Svelte stores (`svelte/store`, framework-agnostic) and plain TS modules in `lib/`; browser globals used are `fetch`, `sessionStorage`/`localStorage`, `WebSocket`, `window.location`. SvelteKit's `$app/*` modules (`goto`, `resolve`) are imported by some stores.

## Goals / Non-Goals

**Goals:**
- One command runs all frontend unit tests locally and in CI in seconds.
- Tests import modules exactly as the app does (`$lib`, relative store imports).
- A small, representative first test set that later changes can extend.

**Non-Goals:**
- Component or end-to-end tests (Testing Library, Playwright). Store and lib logic is where the untested behavior lives; components are thin.
- Coverage thresholds.
- Refactoring stores to be more testable beyond what a test needs.

## Decisions

### D1: Vitest, configured via `vite.config.ts`

Vitest reuses the Vite pipeline, so the `sveltekit()` plugin resolves `$lib` and `$app/*` for tests too. Add a `test` block to `vite.config.ts` (`environment: 'jsdom'`, `include: ['src/**/*.test.ts']`, `globals: false`) rather than a separate `vitest.config.ts`, so there is one source of truth for aliases and plugins.

Alternatives: Jest (needs a separate transform pipeline for SvelteKit aliases and ESM); bun's built-in `bun test` (fast, but no Vite plugin pipeline, so `$lib`/`$app` would need manual aliasing and Svelte-aware transforms would be unavailable for later component tests).

### D2: jsdom environment

Stores touch `sessionStorage`, `window.location` and `fetch`. jsdom provides the first two; `fetch` and `WebSocket` are stubbed per test with `vi.stubGlobal`. happy-dom is faster but has had gaps with `URL`/storage APIs; jsdom is the conservative choice and the suite is small.

### D3: `$app/*` handled by SvelteKit's own Vitest support

`@sveltejs/kit` ships virtual `$app/*` modules that resolve under Vitest; where a store calls `goto`, the test mocks `$app/navigation` with `vi.mock`. No custom alias table.

### D4: Colocated tests, `*.test.ts`

`src/lib/relative-time.test.ts` next to `src/lib/relative-time.ts`, matching the Rust crates' colocated unit tests. `svelte-check`, ESLint and Prettier already cover `src/**`, so tests are linted and type-checked with no extra config; add `vitest/globals` is not used (explicit imports from `vitest`) to keep `tsconfig` unchanged.

### D5: First test set targets the modules with logic and no UI

| Module | What is asserted |
|---|---|
| `lib/relative-time.ts` | boundaries (seconds, minutes, hours, days), future dates |
| `lib/landingResume.ts` | store/consume round trip, consume clears, expired or malformed entries ignored |
| `lib/ws.ts` `createWebSocketUrl` | `http`→`ws`, `https`→`wss`, path joining |
| `stores/tasksStore.ts` | `monitorTask` fires callback once on terminal state, `updateTask` merges into the store |
| `stores/permissionStore.ts` | `hasPermission`: dev mode, wildcard permission, permission present/absent (per-app semantics once `frontend-app-permissions-and-errors` lands) |
| `lib/index.ts` `authenticatedApiCall` | 401 triggers unauthorized handling, non-2xx throws, success returns JSON (mocked `fetch`, mocked `sessionStore`) |

### D6: CI

`bun run test` is added to the frontend job after lint and before build, so a failing test fails the job with the same fast feedback as lint.

## Risks / Trade-offs

- [SvelteKit plugin under Vitest needs `svelte-kit sync` to generate types] → the `test` script runs `svelte-kit sync && vitest run`, like `check` does.
- [Stores hold module-level state between tests] → each test file resets stores in `beforeEach` (stores expose `set`; add a `reset` only where none exists).
- [jsdom adds ~10 MB of dev dependencies] → acceptable; dev-only.
- [Tests written against `hasPermission`'s current any-scope semantics will change when `frontend-app-permissions-and-errors` lands] → that change updates the tests as part of its task 2.1; the two changes should not be applied in parallel on the same store.

## Migration Plan

Single PR. No runtime impact. Rollback is reverting the PR.
