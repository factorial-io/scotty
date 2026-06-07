---
# scotty-wdym
title: Fix stuck release-please and release main as 0.3.0
status: in-progress
type: bug
created_at: 2026-06-07T15:17:09Z
updated_at: 2026-06-07T15:17:09Z
---

release-please never tagged its first release (#805 'chore: release main', commit b5d8967). Root cause: separate-pull-requests:false triggers Merge plugin -> non-componentized branch + retitled PR; getBranchComponent() resolves to package-name 'scotty' but merged PR branch component is undefined -> mismatch -> v0.3.0 never tagged -> 'untagged merged release PRs outstanding' aborts all new release PRs. Fix: set separate-pull-requests:true; reset manifest to 0.2.9 to re-release current main (fixed Cargo.lock/docker) as 0.3.0; relabel #805 to clear pending.
