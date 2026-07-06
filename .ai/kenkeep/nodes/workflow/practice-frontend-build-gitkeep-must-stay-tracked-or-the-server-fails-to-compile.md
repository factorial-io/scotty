---
type: practice
title: frontend/build/.gitkeep must stay tracked or the server fails to compile
description: >-
  include_dir!("frontend/build") panics at compile time when the directory is
  missing from a checkout; local frontend builds can silently delete .gitkeep.
tags:
  - ci
  - frontend
  - build
  - gotcha
kk_schema_version: 3
kk_id: >-
  practice-frontend-build-gitkeep-must-stay-tracked-or-the-server-fails-to-compile
kk_derived_from:
  - '08436e22-ac06-4970-a04c-9e39d3d7bc13:practice:0'
kk_relates_to:
  - practice-git-rules
  - practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks
kk_depends_on: []
kk_confidence: high
---
The scotty server embeds the frontend with `include_dir!("frontend/build")` in `scotty/src/static_files.rs`. A checkout without `frontend/build` fails to compile with a proc-macro panic — CI checkouts contain only tracked files, so `frontend/build/.gitkeep` must remain tracked.

Gotcha: running a frontend build locally rewrites `frontend/build` and removes `.gitkeep`; with jj, the working copy auto-snapshots that deletion into the current commit unnoticed. Check that `.gitkeep` still exists in commits produced while a local frontend build ran.

<!-- kk:related:start -->
# Related

- Related: [practice-git-rules](/workflow/practice-git-rules.md)
- Related: [practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks](/workflow/practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [08436e22-ac06-4970-a04c-9e39d3d7bc13:practice:0](08436e22-ac06-4970-a04c-9e39d3d7bc13:practice:0)
<!-- kk:citations:end -->
