---
type: practice
title: Contributor setup instructions belong in the root README
description: >-
  Per-clone setup steps go in the root README's Developing/Contributing section,
  not in subdirectory READMEs.
tags:
  - documentation
  - readme
  - conventions
kk_schema_version: 3
kk_id: practice-contributor-setup-instructions-belong-in-the-root-readme
kk_derived_from:
  - '802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:1'
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Setup steps a contributor must run after cloning (tool installation, hook generation, local prerequisites) are documented in the root `README.md` under the Developing/Contributing section. Subdirectory READMEs describe their own subsystem but are not the place for clone-time install instructions; they may link out from the root section instead.

<!-- kk:citations:start -->
# Citations

[1] [802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:1](802ace42-0a9d-4291-90e2-56e2ca6b5954:practice:1)
<!-- kk:citations:end -->
