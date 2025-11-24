---
title: Secure OAuth client_secret handling
status: closed
priority: 1
issue_type: task
labels:
- security
- oauth
created_at: 2025-11-24T20:26:58.463333+00:00
updated_at: 2025-11-24T20:34:05.815745+00:00
closed_at: 2025-11-24T20:34:05.815745+00:00
---

# Description

The OAuth client_secret field is currently a plain String with Debug trait, which could expose secrets in logs.

Current issues:
- client_secret in OAuthSettings is Option<String> (scotty-core/src/settings/api_server.rs:31)
- client_secret in OAuthClient is String with Debug derive (scotty/src/oauth/mod.rs:23)
- Debug output would expose the secret in logs
- Used in Basic Auth header construction (device_flow.rs:136)

Required changes:
1. Change client_secret type from String to secrecy::Secret<String> in OAuthSettings
2. Update OAuthClient to use Secret<String> for client_secret  
3. Add custom Debug impl for OAuthClient to redact client_secret
4. Use expose_secret() when needed (e.g., Basic Auth header)
5. Add #[serde(skip_serializing)] to prevent JSON serialization

The secrecy crate is already in the workspace dependencies.

# Acceptance Criteria

- client_secret uses secrecy::Secret<String> type
- OAuthClient has custom Debug impl that redacts secret
- All tests pass
- No client_secret values in debug logs
