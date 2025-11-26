---
title: Document domain-based authorization in AGENTS.md
status: closed
priority: 2
issue_type: task
created_at: 2025-11-26T09:46:36.496336+00:00
updated_at: 2025-11-26T09:56:04.734035+00:00
closed_at: 2025-11-26T09:56:04.734035+00:00
---

# Description

The domain-based authorization feature (PR #594) is missing documentation in AGENTS.md explaining how to use it.

**What needs to be added to AGENTS.md:**
1. How to use domain patterns (@factorial.io syntax)
2. Assignment precedence rules (exact > domain > wildcard)
3. Example use cases and configuration examples
4. Security considerations (what's prevented: subdomain attacks, partial matches)

**Location:** AGENTS.md section on Authorization System

**Example content to add:**
- Domain assignment syntax: '@factorial.io' matches all users from that domain
- Precedence: Exact email match takes priority, then domain match, then wildcard
- Example config showing all three types of assignments
- Note about case-insensitive email matching per RFC 5321

**Files to modify:**
- AGENTS.md (or CLAUDE.md if that's the primary guide)

**Reference:**
- Implementation: scotty/src/services/authorization/casbin.rs
- Tests: scotty/tests/authorization_domain_test.rs
- PR: #594
