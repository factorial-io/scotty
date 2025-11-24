---
title: Add timeout to Docker command execution
status: open
priority: 1
issue_type: task
labels:
- docker
- stability
created_at: 2025-10-26T20:21:10.406194+00:00
updated_at: 2025-11-24T20:17:25.552589+00:00
---

# Description

Docker commands currently have no timeout and could hang indefinitely if Docker daemon becomes unresponsive.

# Design

Location: scotty/src/docker/docker_compose.rs:88

Current code:
```rust
let output = cmd.output()?;  // No timeout
```

Proposed solution:
```rust
use tokio::time::timeout;

let output = timeout(
    Duration::from_secs(300),  // 5 minute timeout
    tokio::process::Command::new("docker-compose")
        .args(command)
        .output()
).await??;
```

Impact: Prevents indefinite hangs on Docker failures
Effort: 1-2 hours
