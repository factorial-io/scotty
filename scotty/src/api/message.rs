use scotty_core::tasks::task_details::TaskDetails;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    Ping,
    Pong,
    AppListUpdated,
    AppInfoUpdated(String),
    TaskListUpdated,
    TaskInfoUpdated(TaskDetails),
    Error(String),
}
