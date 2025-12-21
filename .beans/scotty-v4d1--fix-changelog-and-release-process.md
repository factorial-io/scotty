---
# scotty-v4d1
title: Fix changelog and release process
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Design and implement automated changelog generation with cargo-release.  SOLUTION VERIFIED: - Modified Cargo.toml pre-release-hook to generate global + per-crate changelogs - Uses git-cliff --include-path to filter commits per crate/folder - Generates changelogs for: scotty-core, scotty-types, scotty, scottyctl, ts-generator, frontend - All changelogs contain proper version sections (e.g., ## [0.2.2]) - Hook runs from workspace root and stages all changelogs - Ensures GitHub Actions can find version sections in CHANGELOG.md  TESTED: Dry-run successfully generated all changelogs with correct filtering.  READY FOR: Real release test to verify GitHub Actions integration.
