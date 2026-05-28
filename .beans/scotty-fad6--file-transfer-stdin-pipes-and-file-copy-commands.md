---
# scotty-fad6
title: 'File Transfer: Stdin Pipes and File Copy Commands'
status: completed
type: epic
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2026-05-26T14:55:03Z
---

# Description  Implement comprehensive file transfer capabilities for scottyctl: 1. Binary stdin/stdout piping for shell commands (e.g., scottyctl app:shell db -c mysql < dump.sql.gz) 2. Dedicated file copy command (e.g., scottyctl app:cp myapp:web:/path/file ./local)  This enables critical workflows like database imports, backups, log collection, and asset management.  Implementation in two phases: Phase 1: Binary pipe support for stdin/stdout (enables shell-based file copy) Phase 2: Dedicated app:cp command using Docker copy API (best UX)

## Summary of Changes

Epic delivered via OpenSpec change `app-file-transfer` (single unified design instead of the two original phases).

Phase 2 (`scotty-kqlr`) completed; docs (`scotty-qcv7`) completed; Phase 1 (`scotty-54nc`, WebSocket binary stdin) scrapped because `app:cp -` covers the same use cases without needing a custom WS binary framing protocol. See `openspec/changes/app-file-transfer/design.md` for the transport decision rationale (D1).
