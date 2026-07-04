---
type: practice
title: Local *.localhost subdomains may not auto-resolve to 127.0.0.1
description: >-
  Not all systems resolve wildcard *.localhost subdomains; add explicit
  /etc/hosts entries for each app hostname used in local dev.
tags:
  - local-dev
  - dns
  - traefik
  - gotcha
kk_schema_version: 3
kk_id: practice-localhost-subdomain-dns-gotcha
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
When running Scotty locally with subdomains of `localhost` (e.g. app-derived hostnames like `nginx.my-nginx-test.localhost`), not all systems automatically resolve these to `127.0.0.1`. If a subdomain doesn't resolve, add explicit entries to `/etc/hosts` for each hostname needed, for example:

```
127.0.0.1	localhost scotty.localhost nginx.my-nginx-test.localhost
```
