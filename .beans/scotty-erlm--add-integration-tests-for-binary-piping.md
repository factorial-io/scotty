---
# scotty-erlm
title: Add integration tests for binary piping
status: todo
type: task
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-54nc
---

# Description  Comprehensive tests for stdin/stdout piping with binary data.  **Test cases**: 1. Text file pipe: echo "test" | scottyctl app:shell ... -c "cat" 2. Binary file pipe: cat random.bin | scottyctl app:shell ... -c "wc -c" 3. Compressed file: cat dump.sql.gz | scottyctl app:shell ... -c "zcat | mysql" 4. Large file: 100MB transfer (verify chunking works) 5. Directory backup: scottyctl app:shell ... -c "tar -czf - /data" > backup.tar.gz 6. Authorization failure case 7. Session not found case 8. Malformed binary frame  **Test locations**: - scottyctl/tests/test_binary_stdin.rs (unit tests) - scotty/tests/test_binary_shell_input.rs (integration tests)  **Time estimate**: 2-3 hours
