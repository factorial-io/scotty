---
# scotty-7aau
title: Write unit tests for domain logic
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  Add tests in scotty/src/services/authorization/service.rs: - test_extract_domain() - valid emails, edge cases - test_validate_domain_assignment() - valid and invalid patterns   - test_assignment_resolution_precedence() - exact > domain > wildcard - test_domain_assignment_additive_wildcard() - wildcard always added  Part of domain-based authorization implementation (scotty-5840d)
