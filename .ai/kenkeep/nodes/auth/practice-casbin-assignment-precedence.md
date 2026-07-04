---
type: practice
title: Casbin assignment matching follows exact > domain > wildcard precedence
description: >-
  Exact email match beats domain pattern beats wildcard; wildcard assignments
  are always additive.
tags:
  - authorization
  - casbin
  - security
kk_schema_version: 3
kk_id: practice-casbin-assignment-precedence
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: high
---
Assignment matching in `config/casbin/policy.yaml` is resolved by precedence: exact email (`user@factorial.io`) beats a domain pattern (`@factorial.io`) beats the wildcard (`*`). The wildcard assignment is always additive (it never replaces more specific matches). Domain patterns exist specifically to prevent subdomain attacks, and matching is case-insensitive per RFC 5321.
