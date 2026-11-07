---
# scotty-k2vd
title: Upgrade frontend and docs dependencies with known vulnerabilities
status: todo
type: task
priority: high
created_at: 2026-11-07T09:59:19Z
updated_at: 2026-11-07T09:59:19Z
parent: scotty-uq4t
---

# Description  The first osv-scanner run of the new Security workflow found 42 high or critical findings (CVSS >= 7.0) in `frontend/bun.lock` and `docs/bun.lock`.  Frontend: devalue 5.8.1 (several CVSS 8.2, shipped to the browser), @sveltejs/kit 2.61.1 (medium), vite 8.0.14, undici 8.10.2, nanoid 3.3.12, postcss 8.5.15, brace-expansion 5.0.6, source-map-js 1.2.1 (mostly build-time).  Docs (@factorial/docs toolchain): locutus 2.0.39 (CVSS 9.8), liquidjs 10.27.0, sharp 0.33.5, image-size 1.2.1, markdown-it 14.2.0, js-yaml, linkify-it, brace-expansion 1.1.15, source-map-js. All build-time only.  # Fix  Upgrade direct dependencies, add `overrides` for transitive ones, verify `bun run build`, `bun run check`, `bun run lint` and the frontend tests. For docs, check whether a newer @factorial/docs release fixes the transitive versions.  Afterwards remove `continue-on-error: true` from the `frontend-deps` job in .github/workflows/security.yml.  Reproduce: `go run github.com/google/osv-scanner/v2/cmd/osv-scanner@v2.7.0 scan source --lockfile=frontend/bun.lock --lockfile=docs/bun.lock`
