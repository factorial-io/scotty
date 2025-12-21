---
# scotty-lbdc
title: Refactor test code organization and extract common helpers
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:54Z
parent: scotty-f8ot
---

# Description  Extract common test helpers and utilities into a dedicated test utilities module to reduce duplication across test files.  # Design  Current state: Test code has duplicated setup/teardown logic and helper functions across multiple test files.  Proposed: - Create `scotty/tests/common/mod.rs` for shared test utilities - Extract common fixtures (test users, apps, configurations) - Create builder patterns for test data - Consolidate mock setup functions  Impact: Reduce test code duplication, easier to maintain tests Effort: 4-6 hours
