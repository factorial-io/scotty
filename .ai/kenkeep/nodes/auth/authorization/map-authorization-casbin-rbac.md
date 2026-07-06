---
type: map
title: Authorization uses Casbin RBAC
description: >-
  RBAC via Casbin; config at config/casbin/policy.yaml, impl in
  services/authorization/casbin.rs.
tags:
  - authorization
  - casbin
  - security
  - map
kk_schema_version: 3
kk_id: map-authorization-casbin-rbac
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Scotty's authorization system uses Casbin for RBAC. Config lives in `config/casbin/policy.yaml`, implementation in `scotty/src/services/authorization/casbin.rs`, tests in `scotty/tests/authorization_domain_test.rs`.

Permissions: `view`, `manage`, `create`, `destroy`, `shell`, `logs`, `admin_read`, `admin_write`, `action_read`, `action_write`, `action_manage`, `action_approve`.

The policy file defines `scopes` (named environments/clients), `roles` (permission bundles), and `assignments` (which roles/scopes a user email or pattern gets).
