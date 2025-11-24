---
title: Write integration tests for domain assignments
status: closed
priority: 2
issue_type: task
labels:
- authorization
- enhancement
- testing
created_at: 2025-11-08T17:23:53.043535+00:00
updated_at: 2025-11-24T20:17:25.558374+00:00
closed_at: 2025-11-09T00:11:30.175622+00:00
---

# Description

Create scotty/tests/authorization_domain_test.rs.

Test scenarios:
- Load config with domain assignments
- Mock OAuth users with various email domains
- Verify permission resolution (exact match, domain match, no match)
- Test API endpoints (create/list domain assignments)
- Test validation errors for invalid domain patterns

Part of domain-based authorization implementation (scotty-5840d)
