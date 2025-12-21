---
# scotty-bkeo
title: Optimize wildcard scope expansion to avoid allocation overhead
status: todo
type: task
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:37Z
parent: scotty-lbxn
---

# Description  Wildcard scope expansion allocates a new Vec on every permission check, causing unnecessary overhead for users with wildcard scopes in hot code paths.  Location: scotty/src/services/authorization/service.rs:320-331  Current Implementation: fn expand_wildcard_scopes(&self, scopes: &[String], ...) -> Vec<String> {     if scopes.contains(&"*".to_string()) {         available_scopes.keys().cloned().collect()     } else {         scopes.to_vec()     } }  Performance Issue: - Called on every permission check - Allocates new Vec for users with wildcard scopes - Clones all scope strings from HashMap - Hot path in authorization middleware  Recommended Optimizations: Option 1: Cache expanded scopes per user (invalidate on scope changes) Option 2: Use Cow<[String]> to avoid allocation when no expansion needed Option 3: Return reference to pre-computed scope list  Priority: MEDIUM - Performance optimization Impact: Reduces allocation overhead in authorization hot path  References: PR #467 review from 2025-11-24
