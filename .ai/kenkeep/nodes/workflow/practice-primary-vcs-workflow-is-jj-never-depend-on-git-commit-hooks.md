---
type: practice
title: Primary VCS workflow is jj; never depend on git commit hooks
description: >-
  The maintainer drives this repo with jj, so git commit/pre-commit hooks never
  fire; tooling must work hook-free.
tags:
  - vcs
  - jj
  - git-hooks
  - workflow
kk_schema_version: 3
kk_id: practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks
kk_derived_from:
  - '802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:0'
kk_relates_to:
  - practice-git-rules
  - map-pre-push-hook-cargo-husky
kk_depends_on: []
kk_confidence: high
---
This repository is primarily worked on with `jj` (Jujutsu) rather than raw git. Git commit-time hooks (pre-commit and friends) therefore never fire in the normal workflow, and no process may rely on them.

Any automation that would traditionally live in a commit hook (index regeneration, formatting, generated-file refresh) must instead run explicitly or via the tool's own write path. For the kenkeep knowledge base specifically, indices are rebuilt by the skills after every write, with `npx kenkeep index rebuild` as the manual fallback.

<!-- kk:related:start -->
# Related

- Related: [practice-git-rules](/practice-git-rules.md)
- Related: [map-pre-push-hook-cargo-husky](/map-pre-push-hook-cargo-husky.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:0](802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:0)
<!-- kk:citations:end -->
