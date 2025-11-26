---
title: Add OpenAPI documentation for domain pattern in assignment creation
status: closed
priority: 2
issue_type: task
created_at: 2025-11-26T09:46:44.990570+00:00
updated_at: 2025-11-26T09:56:37.825285+00:00
closed_at: 2025-11-26T09:56:37.825285+00:00
---

# Description

The assignment creation endpoint needs OpenAPI/utoipa annotations documenting the new domain pattern format (@factorial.io).

**Current state:**
The API accepts domain patterns but doesn't document them in the OpenAPI schema.

**What needs to be added:**
1. Update utoipa annotations on assignment creation endpoint
2. Document domain pattern syntax in schema examples
3. Add validation rules (must start with @, must contain dot, etc.)
4. Include example requests showing domain assignments

**Location:**
- scotty/src/api/rest/handlers/admin/assignments.rs

**Example OpenAPI annotation to add:**
\`\`\`rust
/// Create a new role assignment for a user, domain, or wildcard
/// 
/// # User Patterns
/// - Exact email: \`user@factorial.io\` - matches specific user
/// - Domain pattern: \`@factorial.io\` - matches all users from domain (requires @ prefix and valid domain)
/// - Wildcard: \`*\` - matches all users
/// 
/// # Precedence
/// Exact match > Domain match > Wildcard
/// 
/// # Examples
/// - Assign admin role to specific user: \`stephan@factorial.io\`
/// - Assign developer role to all @factorial.io users: \`@factorial.io\`
/// - Assign viewer role to everyone: \`*\`
\`\`\`

**Files to modify:**
- scotty/src/api/rest/handlers/admin/assignments.rs (utoipa annotations)

**Acceptance criteria:**
- OpenAPI schema shows domain pattern examples
- Swagger UI displays the documentation
- Validation rules are documented
- Example requests include domain patterns

**Reference:**
- Validation logic: scotty/src/services/authorization/service.rs (validate_domain_assignment)
- PR: #594
