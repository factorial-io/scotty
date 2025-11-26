---
title: Sanitize scottyctl --server input to accept hostnames
status: open
priority: 3
issue_type: feature
created_at: 2025-11-25T18:06:14.752996+00:00
updated_at: 2025-11-25T18:06:14.752996+00:00
---

# Description

scottyctl should accept server input in multiple formats and normalize it:

**Current behavior:**
- Requires full URL format: `--server https://www.example.com`

**Desired behavior:**
- Accept hostname only: `--server www.example.com` → auto-add `https://`
- Accept full URL: `--server https://www.example.com` → use as-is
- Accept http URLs: `--server http://localhost:21342` → use as-is

**Implementation notes:**
- Add URL normalization in scottyctl argument parsing
- Default to https:// when no scheme provided
- Preserve http:// if explicitly specified (useful for local dev)
