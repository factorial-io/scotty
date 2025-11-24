---
title: Add documentation for hybrid OAuth + bearer token authentication
status: closed
priority: 1
issue_type: task
depends_on:
  scotty-b5863: blocks
created_at: 2025-11-03T10:22:43.917274+00:00
updated_at: 2025-11-24T20:17:25.573059+00:00
closed_at: 2025-11-03T16:17:54.680886+00:00
---

# Description

Users need documentation explaining how to configure the API for hybrid authentication where OAuth is used for human users and bearer tokens for service accounts. Currently there's no user-facing documentation for this capability.

# Design

Add documentation section explaining:
1. The use case: OAuth for users + bearer tokens for service accounts
2. Configuration example showing auth_mode=oauth with bearer_tokens
3. How authentication fallback works (OAuth tried first, bearer second)
4. RBAC configuration for service account identifiers
5. Migration guide for existing OAuth deployments

Location: README.md or docs/authentication.md

# Acceptance Criteria

- Documentation section exists explaining hybrid auth
- Configuration example provided
- Authentication flow explained
- RBAC setup for service accounts documented
- Migration guide included
