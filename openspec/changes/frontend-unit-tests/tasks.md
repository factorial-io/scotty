## 1. Runner

- [x] 1.1 Add `vitest` and `jsdom` as dev dependencies with bun, add `test` (`svelte-kit sync && vitest run`) and `test:watch` scripts, and a `test` block in `vite.config.ts` (jsdom, `src/**/*.test.ts`); verified: `passWithNoTests: true` makes `bun run test` exit 0 with zero tests (Vitest exits 1 by default)
- [x] 1.2 Add a smoke test `src/lib/relative-time.test.ts` importing via `$lib/relative-time`; verify `bun run test` passes and `bun run lint` and `bun run check` still pass with the test file present

## 2. Tests for lib/

- [x] 2.1 `lib/relative-time.test.ts`: seconds/minutes/hours/days boundaries and future dates; verify `bun run test` passes
- [x] 2.2 `lib/landingResume.test.ts` with jsdom `sessionStorage`: store then consume returns the URL and clears it, consume with nothing stored returns null, malformed or expired entries are ignored; verify `bun run test` passes
- [x] 2.3 `lib/ws.test.ts` and `lib/ws-http.test.ts` for `createWebSocketUrl` using the `@vitest-environment-options` docblock to set the jsdom URL (jsdom's `window.location` cannot be stubbed): https→wss with port, http→ws, query string kept; verify `bun run test` passes
- [x] 2.4 `lib/index.test.ts` for `authenticatedApiCall` with `vi.stubGlobal('fetch', ...)` and a mocked session store: success returns parsed JSON, non-2xx throws with status text, 401 invokes the unauthorized handler; verify `bun run test` passes

## 3. Tests for stores/

- [x] 3.1 `stores/tasksStore.test.ts`: `updateTask` merges into the `tasks` store; `monitorTask` invokes its callback exactly once when the task becomes `Finished` or `Failed` and not while `Running`; verify `bun run test` passes
- [x] 3.2 `stores/permissionStore.test.ts`: `hasPermission` returns true in dev auth mode, true for a wildcard permission, true/false for a listed/unlisted permission, and `getAppPermissions` batches results; verify `bun run test` passes

## 4. CI and docs

- [x] 4.1 Add a `Run tests` step (`bun run test`) to the frontend job in `.github/workflows/ci.yml` between lint and build; verified the workflow YAML parses; the step will show on the next PR run
- [x] 4.2 Add kenkeep node `frontend/practice-frontend-unit-tests-vitest` (colocated `*.test.ts`, jsdom, `vi.stubGlobal` for browser APIs, `vi.mock('$app/navigation')`, no component tests yet), run `npx kenkeep index rebuild`; verify no stale-node warning
- [x] 4.3 Create a bean, complete it with a summary, and commit with `jj` as `test(frontend): add Vitest unit tests for stores and lib`; verify `bun run test`, `bun run lint`, `bun run check`, `bun run build` all pass on the commit
