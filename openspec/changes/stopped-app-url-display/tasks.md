# Tasks: stopped-app-url-display

## 1. Component change

- [ ] 1.1 Restructure `frontend/src/components/app-service-button.svelte`: branch on `service.domains.length` (fallback disabled button only when there are no domains), render domain links for all statuses
- [ ] 1.2 Apply muted styling (e.g. `btn-ghost opacity-60`) plus a "service not running" tooltip to links when `service.status !== 'Running'`; keep current styling for running services

## 2. Verification

- [ ] 2.1 Run `bun run check` and `bun run lint` in `frontend/`
- [ ] 2.2 Verify visually in the dashboard (dev server) with one running and one stopped app: stopped app shows dimmed clickable URLs in both light and dark themes; a service without domains still shows the disabled name button
