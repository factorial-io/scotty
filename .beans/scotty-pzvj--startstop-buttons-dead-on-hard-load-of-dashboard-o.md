---
# scotty-pzvj
title: Start/stop buttons dead on hard load of dashboard overview
status: completed
type: bug
created_at: 2026-07-06T20:31:49Z
updated_at: 2026-07-06T20:31:49Z
---

On a direct server load of /dashboard, start/stop buttons stayed disabled until navigating to a detail page and back.

## Summary of Changes
Root cause: only the detail pages and OAuth callback called loadUserPermissions(); the overview never did, so permissionsLoaded stayed false and canManage was false. Fixed by loading permissions in the root layout via a reactive statement on isLoggedIn (frontend/src/routes/+layout.svelte), covering every route on hard load. jj commit kmvltsqw.
