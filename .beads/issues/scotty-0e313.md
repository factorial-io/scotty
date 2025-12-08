---
title: Add get_session_info() to ShellService for authorization
status: open
priority: 1
issue_type: task
depends_on:
  scotty-f4e02: parent-child
created_at: 2025-12-08T23:53:25.857556+00:00
updated_at: 2025-12-08T23:53:25.857556+00:00
---

# Description

Add method to retrieve session metadata for authorization checks.

**Implementation** (scotty/src/docker/services/shell.rs):
```rust
impl ShellService {
    pub async fn get_session_info(&self, session_id: Uuid) -> Option<ShellSessionInfo> {
        let sessions = self.active_sessions.read().await;
        sessions.get(&session_id).map(|s| ShellSessionInfo {
            session_id: s.session_id,
            app_name: s.app_name.clone(),
            service_name: s.service_name.clone(),
            container_id: s.container_id.clone(),
            shell_command: String::new(),
        })
    }
}
```

**Why needed**: Binary frames don't contain app_name, so we need to look up which app the session belongs to for authorization.

**Testing**:
- Unit test: Verify returns correct session info
- Unit test: Returns None for unknown session_id

**Time estimate**: 1 hour
