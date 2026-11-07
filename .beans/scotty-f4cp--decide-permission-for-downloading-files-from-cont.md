---
# scotty-f4cp
title: Decide permission for downloading files from containers
status: todo
type: task
priority: high
created_at: 2026-11-07T10:21:17Z
updated_at: 2026-11-07T10:21:17Z
parent: scotty-uq4t
---

# Description  `GET /api/v1/apps/{app_id}/services/{service}/files` (`scottyctl app:cp` from a container) only requires `view`. It streams any path inside the container as a tar archive, so a read-only viewer can read secrets, `.env` files and database files from running containers. That is close to shell-level read access, while opening a shell requires `shell`. Upload already requires `manage`.  # Decide  Options: require `shell` (consistent with shell access), require `manage` (consistent with upload), or add a dedicated `files` permission. Update the route in scotty/src/api/router.rs, the entry in scotty/tests/fixtures/authorization_matrix.yaml, and docs/content/authorization.md.  Found by the authorization matrix test (openspec change `security-review-and-pentest-plan`).
