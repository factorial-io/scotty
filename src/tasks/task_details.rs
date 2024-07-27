use std::{sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use utoipa::ToResponse;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToResponse, utoipa::ToSchema)]
pub struct TaskDetails {
    pub id: Uuid,
    pub command: String,
    pub state: State,
    pub output: String,
    pub start_time: SystemTime,
    pub finish_time: Option<SystemTime>,
    pub last_read_position: usize,
}

impl Default for TaskDetails {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            command: "".to_string(),
            state: State::Running,
            output: "".to_string(),
            start_time: SystemTime::now(),
            finish_time: None,
            last_read_position: 0,
        }
    }
}

#[derive(Debug)]
pub struct TaskState {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub details: Arc<RwLock<TaskDetails>>,
}
