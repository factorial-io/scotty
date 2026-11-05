---
# scotty-9n7o
title: Autowake stopped apps via basic auth
status: completed
type: feature
priority: normal
created_at: 2026-11-05T16:16:49Z
updated_at: 2026-11-05T16:22:56Z
---

Implement openspec change app-autowake (openspec/changes/app-autowake/tasks.md).

## Summary of Changes

Implemented openspec change app-autowake: AppSettings.autowake, create validation (requires basic_auth), wake_app without notification, per-app WakeGuard in-flight set on AppState, basic-auth-gated autowake branch in landing.rs (401 realm=traefik / self-refreshing 503), scottyctl --autowake, docs and kenkeep node.
