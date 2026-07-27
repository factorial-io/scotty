---
type: practice
title: 'Rate limits are per-instance, not global, across multiple Scotty instances'
description: >-
  In-memory token-bucket rate limiting is per-process; N horizontally-scaled
  instances multiply the effective limit by N.
tags:
  - rate-limiting
  - deployment
  - gotcha
kk_schema_version: 3
kk_id: practice-rate-limiting-is-per-instance-not-global
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
The current rate limiting implementation uses in-memory token bucket counters, which is fast and dependency-free for single-instance deployments, but means limits are per-instance in horizontally-scaled deployments: with 3 instances and a configured 60 req/min limit, the effective global limit is roughly 180 req/min.

For distributed deployments, the recommended mitigation is to enforce global rate limiting at an external layer (load balancer, e.g. Traefik/Nginx rate-limit middleware, or an API gateway) rather than relying on Scotty's per-instance counters.
