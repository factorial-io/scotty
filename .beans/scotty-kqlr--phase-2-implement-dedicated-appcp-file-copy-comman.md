---
# scotty-kqlr
title: 'Phase 2: Implement dedicated app:cp file copy command'
status: completed
type: feature
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2026-05-26T14:54:54Z
parent: scotty-fad6
---

# Description  Add docker cp-like command for intuitive file copying between local and containers.  **Command syntax**: - scottyctl app:cp myapp:web:/path/file ./local-file - scottyctl app:cp ./local-file myapp:web:/path/file - scottyctl app:cp myapp:web:/var/www ./backup-www  **Implementation approach**: Use Docker's native copy API via Bollard (same as docker cp).  **Architecture**: - Server: REST endpoints using bollard::Docker::copy_from_container / upload_to_container - Client: CLI command that creates/extracts tar archives automatically - Protocol: HTTP streaming with tar format (not WebSocket)  **Files to create/modify**: 1. scotty/src/api/rest/handlers/files.rs (new file) 2. scottyctl/src/commands/apps/cp.rs (new file) 3. scottyctl/src/cli.rs: Add CopyCommand to Commands enum 4. scotty/src/api/router.rs: Register new routes  **Dependencies**: tar = "0.4", urlencoding = "2.1"  **Time estimate**: 8-12 hours  **Advantages over shell-based**: - No dependency on cat/tar in container - Preserves permissions and timestamps - Can add progress bars, checksums, resume support - More intuitive UX

## Summary of Changes

Implemented as part of OpenSpec change `app-file-transfer` (commits on bookmark `feat/app-file-transfer`).

Delivered:
- Server endpoints `GET`/`PUT /api/v1/apps/{app_id}/services/{service}/files` using Bollard's `download_from_container` / `upload_to_container` with tar streaming, RBAC (`view`/`manage`), and a counting stream that enforces `SCOTTY__FILES__MAX_TRANSFER_SIZE` (default 1 GiB).
- `scottyctl app:cp` command with `docker cp`-like syntax. Supports `app:service:path`, implicit `app::path` (resolved via `apps/info` public services), and `-` for stdin/stdout pipes.
- Tar pack/unpack helpers using `spawn_blocking` + bounded `mpsc` to bridge the sync `tar` crate.
- 20 unit tests passing; integration tests with Docker placeholders are `#[ignore]`-d.
- Docs in `docs/content/cli.md` and `README.md`.
