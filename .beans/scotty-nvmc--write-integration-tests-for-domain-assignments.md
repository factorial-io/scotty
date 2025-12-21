---
# scotty-nvmc
title: Write integration tests for domain assignments
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:48Z
blocking:
    - scotty-6zas
---

# Description  Create scotty/tests/authorization_domain_test.rs.  Test scenarios: - Load config with domain assignments - Mock OAuth users with various email domains - Verify permission resolution (exact match, domain match, no match) - Test API endpoints (create/list domain assignments) - Test validation errors for invalid domain patterns  Part of domain-based authorization implementation (scotty-5840d)
