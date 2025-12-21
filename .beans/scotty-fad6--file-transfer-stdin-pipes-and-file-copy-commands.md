---
# scotty-fad6
title: 'File Transfer: Stdin Pipes and File Copy Commands'
status: todo
type: epic
priority: critical
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  Implement comprehensive file transfer capabilities for scottyctl: 1. Binary stdin/stdout piping for shell commands (e.g., scottyctl app:shell db -c mysql < dump.sql.gz) 2. Dedicated file copy command (e.g., scottyctl app:cp myapp:web:/path/file ./local)  This enables critical workflows like database imports, backups, log collection, and asset management.  Implementation in two phases: Phase 1: Binary pipe support for stdin/stdout (enables shell-based file copy) Phase 2: Dedicated app:cp command using Docker copy API (best UX)
