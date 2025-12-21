---
# scotty-znml
title: Sanitize scottyctl --server input to accept hostnames
status: todo
type: feature
priority: normal
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T12:44:46Z
---

# Description  scottyctl should accept server input in multiple formats and normalize it:  **Current behavior:** - Requires full URL format: `--server https://www.example.com`  **Desired behavior:** - Accept hostname only: `--server www.example.com` → auto-add `https://` - Accept full URL: `--server https://www.example.com` → use as-is - Accept http URLs: `--server http://localhost:21342` → use as-is  **Implementation notes:** - Add URL normalization in scottyctl argument parsing - Default to https:// when no scheme provided - Preserve http:// if explicitly specified (useful for local dev)
