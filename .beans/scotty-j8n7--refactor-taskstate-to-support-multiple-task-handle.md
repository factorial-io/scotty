---
# scotty-j8n7
title: Refactor TaskState to support multiple task handles
status: todo
type: feature
priority: high
created_at: 2025-12-21T12:44:46Z
updated_at: 2025-12-21T13:33:54Z
parent: scotty-f8ot
---

# Description  Currently TaskState only supports a single JoinHandle, but logical operations (like 'create app') spawn multiple tasks: - State machine orchestration task - Docker compose process task - Potentially multiple service containers  This causes issues: 1. State machine handle is dropped immediately (_handle in helper.rs) 2. Calling add_task() multiple times overwrites the previous entry 3. No way to detect panics in state machine tasks 4. Cannot monitor all subprocess handles  ## Proposed Solution  Refactor TaskState to support multiple handles:  ```rust pub struct TaskState {     pub handles: Vec<Arc<RwLock<tokio::task::JoinHandle<()>>>>,     pub details: Arc<RwLock<TaskDetails>>,     pub primary_handle_index: usize, // Which handle determines task completion } ```  Add TaskManager method: ```rust pub async fn add_task_handle(&self, id: &Uuid, handle: tokio::task::JoinHandle<()>) -> bool ```  ## Benefits - Track all related handles for panic detection - Clean separation: TaskDetails (DTO) unchanged, TaskState (runtime) enhanced - No breaking changes to TypeScript types - Better observability  ## Files to Update - scotty-core/src/tasks/task_details.rs (TaskState definition) - scotty/src/tasks/manager.rs (add_task_handle method, cleanup logic) - scotty/src/docker/helper.rs (store state machine handle)
