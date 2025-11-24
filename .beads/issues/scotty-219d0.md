---
title: Upgrade http/hyper ecosystem to v1.x
status: open
priority: 3
issue_type: chore
labels:
- dependencies
- http
- infrastructure
created_at: 2025-10-26T21:08:24.871193+00:00
updated_at: 2025-11-24T20:17:25.553326+00:00
---

# Description

The http and hyper crates have major version updates (http 0.2 → 1.3, hyper 0.14 → 1.7, http-body 0.4 → 1.0). This is a coordinated ecosystem upgrade.

# Design

Current versions (transitive dependencies):
- http: 0.2.12 → 1.3.1
- http-body: 0.4.6 → 1.0.1
- hyper: 0.14.32 → 1.7.0
- h2: 0.3.27 → 0.4.12

These are foundational HTTP crates used by axum, reqwest, and other dependencies.

Steps:
1. Review hyper 1.0 migration guide and breaking changes
2. Check if current versions of axum/reqwest support hyper 1.x
3. May need to update axum, reqwest, tower-http in coordination
4. Update any direct usage of http/hyper types in code
5. Test all HTTP endpoints (REST API, WebSocket, etc.)
6. Verify middleware and error handling still works
7. Run full integration test suite

Impact: Major HTTP stack upgrade, affects all network communication
Effort: 6-10 hours

Note: This is a significant upgrade that touches the core HTTP stack. Should be done carefully with comprehensive testing. May need to wait for ecosystem crates to fully support hyper 1.x.
