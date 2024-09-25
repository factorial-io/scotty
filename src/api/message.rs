use serde::{Deserialize, Serialize};

use crate::tasks::task_details::TaskDetails;

#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    Ping,
    Pong,
    AppListUpdated,
    AppInfoUpdated(String),
    TaskListUpdated(),
    TaskInfoUpdated(TaskDetails),
    Error(String),
}
