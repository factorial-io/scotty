---
title: 'Phase 2: Implement dedicated app:cp file copy command'
status: open
priority: 2
issue_type: task
depends_on:
  scotty-1836f: parent-child
  scotty-f4e02: blocks
created_at: 2025-12-08T23:53:48.928703+00:00
updated_at: 2025-12-08T23:53:48.928703+00:00
---

# Description

Add docker cp-like command for intuitive file copying between local and containers.

**Command syntax**:
- scottyctl app:cp myapp:web:/path/file ./local-file
- scottyctl app:cp ./local-file myapp:web:/path/file
- scottyctl app:cp myapp:web:/var/www ./backup-www

**Implementation approach**: Use Docker's native copy API via Bollard (same as docker cp).

**Architecture**:
- Server: REST endpoints using bollard::Docker::copy_from_container / upload_to_container
- Client: CLI command that creates/extracts tar archives automatically
- Protocol: HTTP streaming with tar format (not WebSocket)

**Files to create/modify**:
1. scotty/src/api/rest/handlers/files.rs (new file)
2. scottyctl/src/commands/apps/cp.rs (new file)
3. scottyctl/src/cli.rs: Add CopyCommand to Commands enum
4. scotty/src/api/router.rs: Register new routes

**Dependencies**: tar = "0.4", urlencoding = "2.1"

**Time estimate**: 8-12 hours

**Advantages over shell-based**:
- No dependency on cat/tar in container
- Preserves permissions and timestamps
- Can add progress bars, checksums, resume support
- More intuitive UX
