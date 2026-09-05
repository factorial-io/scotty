## Why

The SvelteKit frontend has no test runner. Logic in stores and `lib/` (permission resolution, API error handling, task monitoring, WebSocket URL building, landing-page resume state, relative time formatting) is only verified by `svelte-check` and manual clicking, so regressions such as the silent 403 in `frontend-app-permissions-and-errors` go unnoticed until someone runs into them. That change had to fall back to manual verification for want of a runner.

## What Changes

- Add Vitest as the frontend test runner, configured through the existing `vite.config.ts` so `$lib`, `$app/*` aliases and TypeScript resolve the same way as in the app.
- Add `bun run test` (single run) and `bun run test:watch` scripts; run `bun run test` in the CI frontend job next to lint and build.
- Add a first set of unit tests for the pure or easily mockable modules: `lib/relative-time.ts`, `lib/landingResume.ts`, `lib/ws.ts` (`createWebSocketUrl`), `stores/tasksStore.ts` (`monitorTask`/`updateTask`), `stores/permissionStore.ts` (`hasPermission`), and `lib/index.ts` (`authenticatedApiCall` error handling with a mocked `fetch`).
- Document the convention (colocated `*.test.ts`, `vi.stubGlobal` for `fetch`/storage, no component tests yet) in a kenkeep node.

No user-visible behavior changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Tooling only; `.openspec.yaml` sets `skip_specs: true`.

## Impact

- `frontend/package.json`, `frontend/vite.config.ts` (or a `vitest.config.ts`), `frontend/tsconfig.json`: new dev dependencies (`vitest`, `jsdom` or `happy-dom`), scripts and test globals.
- `.github/workflows/ci.yml`: frontend job runs `bun run test`.
- New `*.test.ts` files next to the modules above.
- `frontend-app-permissions-and-errors`: its tasks 2.1 and 3.1 can gain the unit tests they originally called for once this lands; the two changes are otherwise independent.
