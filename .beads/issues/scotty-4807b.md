---
title: Upgrade rustls ecosystem from 0.21 to 0.23
status: open
priority: 3
issue_type: chore
labels:
- dependencies
- security
- tls
created_at: 2025-10-26T21:08:11.918721+00:00
updated_at: 2025-11-24T20:17:25.549524+00:00
---

# Description

rustls, tokio-rustls, and hyper-rustls have major version updates available (0.21 → 0.23, 0.24 → 0.26, 0.24 → 0.27). These need coordinated updates for TLS support.

# Design

Current versions (transitive dependencies):
- rustls: 0.21.12 → 0.23.34
- tokio-rustls: 0.24.1 → 0.26.4  
- hyper-rustls: 0.24.2 → 0.27.7

These are currently pulled in transitively through reqwest and other dependencies.

Steps:
1. Review rustls 0.23 changelog for breaking changes
2. Check if reqwest needs update to support rustls 0.23
3. Update any direct dependencies on rustls/tokio-rustls
4. Ensure all TLS connections still work properly
5. Test HTTPS endpoints and WebSocket over TLS
6. Verify certificate validation still works

Impact: TLS/SSL implementation changes, better security and performance
Effort: 4-6 hours

Note: This may require updating other HTTP-related crates in coordination.
