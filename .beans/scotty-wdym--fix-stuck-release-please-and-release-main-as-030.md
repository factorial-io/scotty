---
# scotty-wdym
title: Fix stuck release-please and release main as 0.3.0
status: completed
type: bug
priority: normal
created_at: 2026-06-07T15:17:09Z
updated_at: 2026-06-07T15:49:28Z
---

release-please never tagged its first release (#805 'chore: release main', commit b5d8967). Root cause: separate-pull-requests:false triggers Merge plugin -> non-componentized branch + retitled PR; getBranchComponent() resolves to package-name 'scotty' but merged PR branch component is undefined -> mismatch -> v0.3.0 never tagged -> 'untagged merged release PRs outstanding' aborts all new release PRs. Fix: set separate-pull-requests:true; reset manifest to 0.2.9 to re-release current main (fixed Cargo.lock/docker) as 0.3.0; relabel #805 to clear pending.

## Summary of Changes

Root cause: release-please's first release attempt (#805) got stuck. `separate-pull-requests: false` ran the Merge plugin for the single root package, producing a non-componentized release branch and a `chore: release main` title. In `Strategy.buildRelease()` the merged PR's branch component was `undefined` while `getBranchComponent()` resolved to package-name `scotty` (it ignores `include-component-in-tag`), so the component mismatch meant v0.3.0 was never tagged. The leftover merged-but-untagged PR then tripped 'untagged, merged release PRs outstanding - aborting' on every run, blocking all new release PRs.

Fix (verified vs release-please v17.6.0 source):
- Set `separate-pull-requests: true` (PR #825) so release-please uses a componentized branch matching the configured component and titles the PR `chore(main): release <version>`.
- Reset `.release-please-manifest.json` to 0.2.9 so a fresh 0.3.0 was proposed against current main (with the Cargo.lock/Docker fix), not the broken b5d8967.
- Cleared the stale `autorelease: pending` label from #805.

Outcome: PR #826 (`chore(main): release 0.3.0`) merged; tag v0.3.0 + GitHub Release created at c5f92b55; Docker image, scottyctl binaries (4 targets), and Homebrew formula all published successfully.
