---
# scotty-yk8x
title: Simplify resolve_user_assignments by removing duplicate precedence logic
status: completed
type: task
priority: normal
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:47Z
---

# Description  The implementation of domain-based authorization introduced duplicate logic between resolve_user_assignments() and Casbin's user_match() function. Both implement the same precedence rules (exact > domain > wildcard).  **Current State:** - resolve_user_assignments() has 3-step precedence logic (lines scotty/src/services/authorization/service.rs) - Casbin user_match() has identical 3-step precedence logic (scotty/src/services/authorization/casbin.rs) - This duplication was unintentional - the goal was to simplify, not duplicate  **Root Cause:** resolve_user_assignments() is called from multiple places that don't go through Casbin enforcer: - get_user_scopes_with_permissions() - get_user_permissions() - Admin API endpoints  **Proposed Solution:** 1. Remove precedence logic from resolve_user_assignments() - make it return ALL matching assignments (exact + domain + wildcard) without filtering 2. Let Casbin matcher handle precedence via user_match() for permission checks 3. Refactor get_user_scopes_with_permissions() and get_user_permissions() to use Casbin for filtering instead of resolve_user_assignments() 4. Document that the source of truth for precedence is the Casbin matcher  **Benefits:** - Single source of truth for precedence rules - Simpler resolve_user_assignments() function - Easier to maintain and test - Aligns with original simplification goal  **Files to modify:** - scotty/src/services/authorization/service.rs (resolve_user_assignments, get_user_permissions, get_user_scopes_with_permissions) - Tests may need updates to reflect new behavior  **Acceptance Criteria:** - resolve_user_assignments() no longer implements precedence logic - All tests pass - Precedence behavior remains correct via Casbin matcher - No breaking changes to authorization behavior
