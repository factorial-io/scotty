---
# scotty-mfd2
title: Fix bearer token authentication timing side-channel
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T13:33:23Z
parent: scotty-uq4t
---

# Description  Bearer token authentication order creates timing difference between invalid bearer token vs invalid OAuth token responses, allowing attackers to enumerate valid bearer token identifiers.  Location: scotty/src/api/basic_auth.rs:47-50  Current Implementation: // Try bearer token authentication first (fast HashMap lookup) if let Some(user) = authorize_bearer_user(state.clone(), auth_header, false).await {     Some(user) } else {     // Not a bearer token, try OAuth validation (network call) }  Security Issue: - Bearer token check is fast (HashMap lookup) - OAuth check is slow (network call to OIDC provider) - Timing difference creates side-channel attack vector - Attacker can distinguish token types by response time  Recommended Fixes: Option 1: Add small random delay (jitter) to bearer token failures to normalize timing Option 2: Check token format first (e.g., JWT structure for OAuth) to route without attempting authentication Option 3: Ensure both paths take similar time  Priority: HIGH - Security vulnerability (timing attack) Severity: Information disclosure via timing side-channel  References: PR #467 review from 2025-11-24
