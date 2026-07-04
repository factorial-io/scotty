---
type: map
title: Which files under config/ are committed vs git-ignored
description: >-
  config/*.example and casbin/model.conf are committed templates; default.yaml,
  local.yaml, and casbin/policy.yaml hold real values and are meant to stay out
  of git (policy.yaml only if it has no secrets).
tags:
  - config
  - casbin
  - git
  - structure
kk_schema_version: 3
kk_id: map-config-directory-git-tracking
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Under `config/`: `default.yaml.example` and `casbin/policy.yaml.example` are templates meant to be committed; `casbin/model.conf` (the RBAC model) is safe to commit as-is. `default.yaml` and `local.yaml` hold real, potentially secret values and are git-ignored. `casbin/policy.yaml` (the actual RBAC policy) can be committed only if it contains no secrets, since it's derived from `policy.yaml.example`. `blueprints/*.yaml` app blueprints are safe to commit.
