---
title: Add Traefik to observability docker-compose for .ddev.site URLs
status: open
priority: 3
issue_type: task
created_at: 2025-10-25T00:59:32.096433+00:00
updated_at: 2025-11-24T20:17:25.559242+00:00
---

# Description

The observability stack uses Traefik labels for routing (grafana.ddev.site, jaeger.ddev.site, vm.ddev.site) but requires external Traefik to be running from apps/traefik. This creates a dependency and extra setup step.

Add Traefik service to observability/docker-compose.yml so the stack works out-of-the-box without requiring apps/traefik to be running.

# Design

Options:
1. Add Traefik service to observability/docker-compose.yml
   - Include network configuration
   - Configure dashboard access
   - Ensure no port conflicts with main Traefik

2. Share network with existing apps/traefik
   - Create external network
   - Less duplication but still requires apps/traefik

Recommended: Option 1 - self-contained observability stack

Implementation:
- Add Traefik service to observability/docker-compose.yml
- Use different ports than apps/traefik (80/443 vs 8080/8443)
- Configure proxy network
- Update README.md to mention this works standalone

# Acceptance Criteria

- grafana.ddev.site, jaeger.ddev.site, vm.ddev.site work without apps/traefik running
- cd observability && docker-compose up -d brings up full working stack
- No port conflicts with apps/traefik
- Documentation updated
