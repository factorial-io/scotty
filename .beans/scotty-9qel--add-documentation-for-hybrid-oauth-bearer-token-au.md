---
# scotty-9qel
title: Add documentation for hybrid OAuth + bearer token authentication
status: completed
type: task
priority: critical
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:47Z
---

# Description  Users need documentation explaining how to configure the API for hybrid authentication where OAuth is used for human users and bearer tokens for service accounts. Currently there's no user-facing documentation for this capability.  # Design  Add documentation section explaining: 1. The use case: OAuth for users + bearer tokens for service accounts 2. Configuration example showing auth_mode=oauth with bearer_tokens 3. How authentication fallback works (OAuth tried first, bearer second) 4. RBAC configuration for service account identifiers 5. Migration guide for existing OAuth deployments  Location: README.md or docs/authentication.md  # Acceptance Criteria  - Documentation section exists explaining hybrid auth - Configuration example provided - Authentication flow explained - RBAC setup for service accounts documented - Migration guide included
