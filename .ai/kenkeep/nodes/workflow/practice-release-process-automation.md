---
type: practice
title: Releases are fully automated via release-please
description: >-
  Never manually bump versions or edit the changelog; conventional-commit type
  drives the version bump; merging the standing release PR performs the release.
tags:
  - release
  - versioning
  - ci
kk_schema_version: 3
kk_id: practice-release-process-automation
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty uses release-please (PR-driven, fully in CI) for releases. Do not manually bump versions or edit changelogs. Commits must follow conventional-commit format (`feat:`, `fix:`, `feat!:`/`BREAKING CHANGE:` for majors) — the commit type determines the version bump and changelog section.

On every push to `main`, the `release-please` workflow maintains a standing release PR that bumps the shared workspace version (`[workspace.package].version` in `Cargo.toml`) and regenerates `CHANGELOG.md`. Merging that release PR is the release: release-please creates the `vX.Y.Z` tag + GitHub Release, then the same workflow uploads `scottyctl` binaries, bumps the Homebrew tap, and publishes the versioned Docker image. Config lives in `release-please-config.json` and `.release-please-manifest.json`; a pre-push hook via `cargo-husky` enforces local quality checks.
