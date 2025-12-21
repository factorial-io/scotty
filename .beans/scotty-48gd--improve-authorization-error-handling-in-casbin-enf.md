---
# scotty-48gd
title: Improve authorization error handling in Casbin enforcement
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T13:33:24Z
parent: scotty-uq4t
---

# Description  Authorization service silently denies access on Casbin errors using unwrap_or(false), potentially hiding system issues.  # Design  Location: scotty/src/services/authorization/service.rs:182  Current code: ```rust let result = enforcer     .enforce(vec![user, app, action_str])     .unwrap_or(false); ```  Proposed solution: ```rust let result = enforcer     .enforce(vec![user, app, action_str])     .map_err(|e| {         error!("Casbin enforce error: {}", e);         crate::metrics::authorization::record_error();     })     .unwrap_or(false); ```  Impact: Better visibility into authorization failures Effort: 30 minutes
