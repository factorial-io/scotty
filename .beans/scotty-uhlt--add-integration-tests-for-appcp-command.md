---
# scotty-uhlt
title: Add integration tests for app:cp command
status: todo
type: task
priority: high
created_at: 2025-12-21T12:44:47Z
updated_at: 2025-12-21T12:44:48Z
parent: scotty-kqlr
---

# Description  End-to-end tests for file copy functionality.  **Test scenarios**: 1. Upload single file to container 2. Download single file from container 3. Upload directory to container (verify tar extraction) 4. Download directory from container (verify tar creation) 5. Binary file transfer (verify no corruption) 6. Large file transfer (100MB+) 7. Permission preservation (verify file modes match) 8. Error cases:    - Container not found    - Service not found    - Path doesn't exist in container    - Permission denied (authorization)    - Disk full scenario  **Comparison with shell-based approach**: - Verify app:cp preserves permissions better than cat - Verify app:cp works without cat/tar in container - Performance comparison for large transfers  **Test file**: scottyctl/tests/test_file_copy.rs  **Time estimate**: 2-3 hours
