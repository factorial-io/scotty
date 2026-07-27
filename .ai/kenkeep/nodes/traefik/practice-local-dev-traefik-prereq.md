---
type: practice
title: Start Traefik before local development
description: >-
  Local dev requires Traefik running via docker compose in apps/traefik as a
  prerequisite.
tags:
  - local-dev
  - traefik
  - prerequisites
kk_schema_version: 3
kk_id: practice-local-dev-traefik-prereq
kk_derived_from: []
kk_relates_to: []
kk_depends_on: []
kk_confidence: medium
---
Before local development, start Traefik: `cd apps/traefik && docker compose up -d`. This is listed as a prerequisite alongside the server/scottyctl/frontend dev commands.
