use std::{sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema, utoipa::ToResponse)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToResponse, utoipa::ToSchema)]
pub struct TaskDetails {
    pub id: Uuid,
    pub command: String,
    pub state: State,
    pub stdout: String,
    pub stderr: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
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
            stdout: "".to_string(),
            stderr: "".to_string(),
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
            stdout: "".to_string(),
            stderr: "".to_string(),
            start_time: chrono::DateTime::from(SystemTime::now()),
            finish_time: None,
            last_exit_code: None,
            app_name,
        }
    }

    pub fn eprint(&mut self, msg: impl AsRef<str>) {
        self.stderr.push_str(msg.as_ref());
    }

    pub fn eprintln(&mut self, msg: impl AsRef<str>) {
        self.eprint(msg);
        self.eprint("\n");
    }
    pub fn print(&mut self, msg: impl AsRef<str>) {
        self.stdout.push_str(msg.as_ref());
    }

    pub fn println(&mut self, msg: impl AsRef<str>) {
        self.print(msg);
        self.print("\n");
    }
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    pub details: Arc<RwLock<TaskDetails>>,
}
