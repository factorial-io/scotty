---
type: map
title: Scotty supports Traefik and legacy Haproxy-config load balancers
description: >-
  Traefik is the primary supported load balancer; haproxy-config is
  legacy/deprecated and lacks robots-blocking support.
tags:
  - scotty
  - traefik
  - haproxy
  - load-balancer
kk_schema_version: 3
kk_id: map-load-balancer-support
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Scotty generates `compose.override.yml` labels/env for two load balancers. For Traefik, it creates labels to route traffic to public services and, depending on settings, adds basic auth or robots-blocking config; custom middlewares are declared in the `config` directory and referenced via the `--middleware` option of `app:create`. For haproxy-config (the legacy setup, https://github.com/factorial-io/haproxy-config), Scotty creates the override to route traffic via environment variables, but it does not support preventing robots from indexing the app, and this support will not be continued going forward since haproxy-config is deprecated.
