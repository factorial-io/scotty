---
# scotty-49nh
title: Show URLs for stopped apps in muted style
status: in-progress
type: feature
priority: normal
created_at: 2026-07-06T20:05:36Z
updated_at: 2026-07-06T20:06:26Z
---

Implement openspec change stopped-app-url-display: app-service-button.svelte renders domain links for all statuses, muted style + tooltip when not Running, fallback button only when no domains.

- [x] Restructure app-service-button.svelte to branch on domains
- [x] Muted styling + tooltip for non-running services
- [x] bun run check + lint
- [ ] Visual verification
