---
title: Add integration tests for binary piping
status: open
priority: 2
issue_type: task
depends_on:
  scotty-f4e02: parent-child
created_at: 2025-12-08T23:53:36.455558+00:00
updated_at: 2025-12-08T23:53:36.455558+00:00
---

# Description

Comprehensive tests for stdin/stdout piping with binary data.

**Test cases**:
1. Text file pipe: echo "test" | scottyctl app:shell ... -c "cat"
2. Binary file pipe: cat random.bin | scottyctl app:shell ... -c "wc -c"
3. Compressed file: cat dump.sql.gz | scottyctl app:shell ... -c "zcat | mysql"
4. Large file: 100MB transfer (verify chunking works)
5. Directory backup: scottyctl app:shell ... -c "tar -czf - /data" > backup.tar.gz
6. Authorization failure case
7. Session not found case
8. Malformed binary frame

**Test locations**:
- scottyctl/tests/test_binary_stdin.rs (unit tests)
- scotty/tests/test_binary_shell_input.rs (integration tests)

**Time estimate**: 2-3 hours
