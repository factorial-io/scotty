---
title: Refactor TaskState to support multiple task handles
status: open
priority: 2
issue_type: feature
labels:
- architecture
- enhancement
created_at: 2025-11-04T21:22:20.154241+00:00
updated_at: 2025-11-24T20:17:25.574852+00:00
---

# Description

Currently TaskState only supports a single JoinHandle, but logical operations (like 'create app') spawn multiple tasks:
- State machine orchestration task
- Docker compose process task
- Potentially multiple service containers

This causes issues:
1. State machine handle is dropped immediately (_handle in helper.rs)
2. Calling add_task() multiple times overwrites the previous entry
3. No way to detect panics in state machine tasks
4. Cannot monitor all subprocess handles

## Proposed Solution

Refactor TaskState to support multiple handles:

```rust
pub struct TaskState {
    pub handles: Vec<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    pub details: Arc<RwLock<TaskDetails>>,
    pub primary_handle_index: usize, // Which handle determines task completion
}
```

Add TaskManager method:
```rust
pub async fn add_task_handle(&self, id: &Uuid, handle: tokio::task::JoinHandle<()>) -> bool
```

## Benefits
- Track all related handles for panic detection
- Clean separation: TaskDetails (DTO) unchanged, TaskState (runtime) enhanced
- No breaking changes to TypeScript types
- Better observability

## Files to Update
- scotty-core/src/tasks/task_details.rs (TaskState definition)
- scotty/src/tasks/manager.rs (add_task_handle method, cleanup logic)
- scotty/src/docker/helper.rs (store state machine handle)
