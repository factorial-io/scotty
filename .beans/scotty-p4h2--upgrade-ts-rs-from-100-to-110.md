---
# scotty-p4h2
title: Upgrade ts-rs from 10.0 to 11.0
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:50:26Z
---

# Description  ts-rs has a major version update available (10.0 → 11.0) that should be evaluated and applied. This affects TypeScript type generation.  # Design  Location: scotty-types/Cargo.toml:21  Current: ts-rs = { version = "10.0", features = ["chrono-impl", "uuid-impl"] } Target: ts-rs = { version = "11.0", features = ["chrono-impl", "uuid-impl"] }  Steps: 1. Review ts-rs 11.0 changelog for breaking changes 2. Update version in scotty-types/Cargo.toml 3. Run TypeScript generation and verify output 4. Test that generated TypeScript types are compatible with frontend 5. Update any code that uses ts-rs macros if needed  Impact: May change generated TypeScript types Effort: 2-4 hours
