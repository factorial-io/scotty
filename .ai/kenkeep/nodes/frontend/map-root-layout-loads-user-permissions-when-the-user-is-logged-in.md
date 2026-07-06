---
type: map
title: Root layout loads user permissions when the user is logged in
description: >-
  frontend/src/routes/+layout.svelte reactively calls loadUserPermissions() on
  isLoggedIn; permission-gated UI derives from permissionsLoaded.
tags:
  - frontend
  - permissions
  - svelte
kk_schema_version: 3
kk_id: map-root-layout-loads-user-permissions-when-the-user-is-logged-in
kk_derived_from:
  - '08436e22-ac06-4970-a04c-9e39d3d7bc13:map:2'
kk_relates_to:
  - map-frontend-src-layout
kk_depends_on: []
kk_confidence: high
---
The SvelteKit root layout runs `$: if ($isLoggedIn) loadUserPermissions()`, so the shared permission store is populated on any hard page load once auth initializes (`isLoggedIn` is also true in dev auth mode). Permission-gated components (e.g. the start/stop app buttons) derive their enabled state from `permissionsLoaded` plus `hasPermission()`; pages must not rely on some other route having populated the permission store first. The app/service detail pages retain their own guarded `loadUserPermissions()` calls, which are no-ops once the layout has loaded permissions.

<!-- kk:related:start -->
# Related

- Related: [map-frontend-src-layout](/frontend/map-frontend-src-layout.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [08436e22-ac06-4970-a04c-9e39d3d7bc13:map:2](08436e22-ac06-4970-a04c-9e39d3d7bc13:map:2)
<!-- kk:citations:end -->
