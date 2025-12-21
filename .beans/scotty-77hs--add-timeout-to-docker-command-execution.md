---
# scotty-77hs
title: Add timeout to Docker command execution
status: todo
type: task
priority: critical
created_at: 2025-12-21T12:44:45Z
updated_at: 2025-12-21T12:44:45Z
---

# Description  Docker commands currently have no timeout and could hang indefinitely if Docker daemon becomes unresponsive.  # Design  Location: scotty/src/docker/docker_compose.rs:88  Current code: ```rust let output = cmd.output()?;  // No timeout ```  Proposed solution: ```rust use tokio::time::timeout;  let output = timeout(     Duration::from_secs(300),  // 5 minute timeout     tokio::process::Command::new("docker-compose")         .args(command)         .output() ).await??; ```  Impact: Prevents indefinite hangs on Docker failures Effort: 1-2 hours
