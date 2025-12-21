---
# scotty-yk98
title: Add domain extraction and validation utilities
status: completed
type: task
priority: high
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Create helper functions in scotty/src/services/authorization/service.rs: - extract_domain(email: &str) -> Option<String> - validate_domain_assignment(user_id: &str) -> Result<(), String>  Validation rules: - Must start with @ - At least one char after @ - Must contain a dot - No additional @ characters  Part of domain-based authorization implementation (scotty-5840d)
