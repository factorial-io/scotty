use std::{sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
#[cfg(feature = "ts-rs")]
use ts_rs::TS;
use uuid::Uuid;

use crate::output::TaskOutput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, utoipa::ToResponse)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(
    feature = "ts-rs",
    ts(export, export_to = "../frontend/src/generated/")
)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToResponse, utoipa::ToSchema)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(
    feature = "ts-rs",
    ts(export, export_to = "../frontend/src/generated/")
)]
pub struct TaskDetails {
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub id: Uuid,
    pub command: String,
    pub state: State,
    #[cfg_attr(feature = "ts-rs", ts(type = "string"))]
    pub start_time: chrono::DateTime<chrono::Utc>,
    #[cfg_attr(feature = "ts-rs", ts(type = "string | null"))]
    pub finish_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_exit_code: Option<i32>,
    pub app_name: Option<String>,
}

impl Default for TaskDetails {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            command: "".to_string(),
            state: State::Running,
            start_time: chrono::DateTime::from(SystemTime::now()),
            finish_time: None,
            last_exit_code: None,
            app_name: None,
        }
    }
}

impl TaskDetails {
    pub fn new(command: String, app_name: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            command,
            state: State::Running,
            start_time: chrono::DateTime::from(SystemTime::now()),
            finish_time: None,
            last_exit_code: None,
            app_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    pub details: Arc<RwLock<TaskDetails>>,
    pub output: Arc<RwLock<TaskOutput>>,
}
