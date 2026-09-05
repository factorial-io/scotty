## 1. Runner

- [ ] 1.1 Add `vitest` and `jsdom` as dev dependencies with bun, add `test` (`svelte-kit sync && vitest run`) and `test:watch` scripts, and a `test` block in `vite.config.ts` (jsdom, `src/**/*.test.ts`); verify `bun run test` runs with zero tests and exits 0
- [ ] 1.2 Add a smoke test `src/lib/relative-time.test.ts` importing via `$lib/relative-time`; verify `bun run test` passes and `bun run lint` and `bun run check` still pass with the test file present

## 2. Tests for lib/

- [ ] 2.1 `lib/relative-time.test.ts`: seconds/minutes/hours/days boundaries and future dates; verify `bun run test` passes
- [ ] 2.2 `lib/landingResume.test.ts` with jsdom `sessionStorage`: store then consume returns the URL and clears it, consume with nothing stored returns null, malformed or expired entries are ignored; verify `bun run test` passes
- [ ] 2.3 `lib/ws.test.ts` for `createWebSocketUrl` with stubbed `window.location`: http→ws, https→wss, absolute and relative paths; verify `bun run test` passes
- [ ] 2.4 `lib/index.test.ts` for `authenticatedApiCall` with `vi.stubGlobal('fetch', ...)` and a mocked session store: success returns parsed JSON, non-2xx throws with status text, 401 invokes the unauthorized handler; verify `bun run test` passes

## 3. Tests for stores/

- [ ] 3.1 `stores/tasksStore.test.ts`: `updateTask` merges into the `tasks` store; `monitorTask` invokes its callback exactly once when the task becomes `Finished` or `Failed` and not while `Running`; verify `bun run test` passes
- [ ] 3.2 `stores/permissionStore.test.ts`: `hasPermission` returns true in dev auth mode, true for a wildcard permission, true/false for a listed/unlisted permission, and `getAppPermissions` batches results; verify `bun run test` passes

## 4. CI and docs

- [ ] 4.1 Add a `Run tests` step (`bun run test`) to the frontend job in `.github/workflows/ci.yml` between lint and build; verify the workflow file parses (`gh workflow view ci.yml` or a YAML lint) and the step appears in the next PR run
- [ ] 4.2 Add kenkeep node `frontend/practice-frontend-unit-tests-vitest` (colocated `*.test.ts`, jsdom, `vi.stubGlobal` for browser APIs, `vi.mock('$app/navigation')`, no component tests yet), run `npx kenkeep index rebuild`; verify no stale-node warning
- [ ] 4.3 Create a bean, complete it with a summary, and commit with `jj` as `test(frontend): add Vitest unit tests for stores and lib`; verify `bun run test`, `bun run lint`, `bun run check`, `bun run build` all pass on the commit
