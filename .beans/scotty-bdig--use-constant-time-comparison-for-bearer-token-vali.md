---
# scotty-bdig
title: Use constant-time comparison for bearer token validation
status: completed
type: bug
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:44Z
---

# Description  Bearer token comparison uses standard equality operator, making it vulnerable to timing attacks.  # Design  Location: scotty/src/api/auth_core.rs:173-182  Current code: ```rust fn find_token_identifier(shared_app_state: &SharedAppState, token: &str) -> Option<String> {     for (identifier, configured_token) in &shared_app_state.settings.api.bearer_tokens {         if configured_token == token {  // NOT constant-time!             return Some(identifier.clone());         }     }     None } ```  Vulnerability: Standard string comparison short-circuits on first mismatch, allowing attackers to reconstruct valid bearer tokens through timing analysis.  Recommendation: Use constant-time comparison from subtle crate: ```rust use subtle::ConstantTimeEq;  if token.as_bytes().ct_eq(configured_token.as_bytes()).into() {     return Some(identifier.clone()); } ```  Add subtle to Cargo.toml: `subtle = "2.5"`  # Notes  PR #524 updated with complete fix:  Initial commit (29845ae): - Fixed timing attack in auth_core.rs find_token_identifier() - Added subtle crate v2.6 for constant-time comparison  Second commit (c3328f1): - Fixed timing attack in login.rs login_handler()   - Applied constant-time comparison to bearer token login - Addresses PR review feedback  All token comparison points now use constant-time operations. All 184 tests passing (110 scotty + 51 scotty-core + 20 scotty-types + 3 scottyctl)  Branch: fix/scotty-2cca4-constant-time-token-comparison PR: https://github.com/factorial-io/scotty/pull/524
