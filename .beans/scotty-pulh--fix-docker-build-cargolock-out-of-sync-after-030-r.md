---
# scotty-pulh
title: 'Fix Docker build: Cargo.lock out of sync after 0.3.0 release'
status: completed
type: bug
priority: high
created_at: 2026-06-07T14:42:52Z
updated_at: 2026-06-07T14:43:26Z
---

release-please bumped workspace version 0.2.9 to 0.3.0 in Cargo.toml but did not update Cargo.lock, breaking 'cargo run --locked' in the ts-generator Docker stage.

## Summary of Changes

Root cause: release-please commit b5d8967 bumped workspace version 0.2.9 to 0.3.0 in Cargo.toml but left Cargo.lock pinning the five local crates at 0.2.9. Docker ts-generator stage runs 'cargo run --locked' and refused the stale lock.

Fix: ran 'cargo update -p scotty --precise 0.3.0' which bumped only the 5 workspace members in Cargo.lock. PR #823.
