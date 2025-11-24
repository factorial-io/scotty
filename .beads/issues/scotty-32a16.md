---
title: Add domain extraction and validation utilities
status: closed
priority: 2
issue_type: task
labels:
- authorization
- enhancement
created_at: 2025-11-08T17:23:27.922409+00:00
updated_at: 2025-11-24T20:17:25.543870+00:00
closed_at: 2025-11-09T00:11:11.155807+00:00
---

# Description

Create helper functions in scotty/src/services/authorization/service.rs:
- extract_domain(email: &str) -> Option<String>
- validate_domain_assignment(user_id: &str) -> Result<(), String>

Validation rules:
- Must start with @
- At least one char after @
- Must contain a dot
- No additional @ characters

Part of domain-based authorization implementation (scotty-5840d)
