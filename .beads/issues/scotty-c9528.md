---
title: Refactor test code organization and extract common helpers
status: open
priority: 1
issue_type: chore
labels:
- maintainability
- testing
created_at: 2025-10-26T20:21:10.906880+00:00
updated_at: 2025-11-24T20:17:25.557856+00:00
---

# Description

Extract common test helpers and utilities into a dedicated test utilities module to reduce duplication across test files.

# Design

Current state: Test code has duplicated setup/teardown logic and helper functions across multiple test files.

Proposed:
- Create `scotty/tests/common/mod.rs` for shared test utilities
- Extract common fixtures (test users, apps, configurations)
- Create builder patterns for test data
- Consolidate mock setup functions

Impact: Reduce test code duplication, easier to maintain tests
Effort: 4-6 hours
