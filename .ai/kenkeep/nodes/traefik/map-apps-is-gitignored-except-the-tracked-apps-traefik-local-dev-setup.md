---
type: map
title: /apps is gitignored except the tracked apps/traefik local-dev setup
description: >-
  .gitignore excludes /apps/* but re-includes /apps/traefik (compose plus
  dynamic file-provider config) so the local-dev Traefik setup ships with the
  repo.
tags:
  - gitignore
  - traefik
  - local-dev
kk_schema_version: 3
kk_id: map-apps-is-gitignored-except-the-tracked-apps-traefik-local-dev-setup
kk_derived_from:
  - '08436e22-ac06-4970-a04c-9e39d3d7bc13:map:1'
kk_relates_to:
  - practice-local-dev-traefik-prereq
  - practice-default-backend-configuration
kk_depends_on: []
kk_confidence: high
---
The apps root folder (`./apps`) holds runtime app instances and is gitignored via `/apps/*`, with the exception `!/apps/traefik` keeping the local-dev Traefik setup tracked: `docker-compose.yml` plus `dynamic/scotty-default-backend.yml` — a Traefik file-provider catch-all router (priority 1, entrypoint `web`) forwarding unmatched domains to Scotty on the host at `http://host.docker.internal:21342` (with `extra_hosts: host-gateway` for Linux).

New files created elsewhere under `apps/` are silently untracked; anything there that must ship with the repo needs its own un-ignore rule.

<!-- kk:related:start -->
# Related

- Related: [practice-local-dev-traefik-prereq](/traefik/practice-local-dev-traefik-prereq.md)
- Related: [practice-default-backend-configuration](/traefik/practice-default-backend-configuration.md)
<!-- kk:related:end -->

<!-- kk:citations:start -->
# Citations

[1] [08436e22-ac06-4970-a04c-9e39d3d7bc13:map:1](08436e22-ac06-4970-a04c-9e39d3d7bc13:map:1)
<!-- kk:citations:end -->
