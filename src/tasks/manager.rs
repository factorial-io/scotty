use std::{collections::HashMap, sync::Arc, time::SystemTime};

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::instrument;
use utoipa::ToResponse;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TaskManager {
    processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum State {
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToResponse, utoipa::ToSchema)]
pub struct TaskDetails {
    id: Uuid,
    command: String,
    state: State,
    output: String,
    start_time: SystemTime,
    finish_time: Option<SystemTime>,
    last_read_position: usize,
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
struct TaskState {
    handle: Option<tokio::task::JoinHandle<()>>,
    details: Arc<RwLock<TaskDetails>>,
}

impl TaskManager {
    pub fn new() -> Self {
        let manager = Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        };

        manager
    }

    pub async fn get_task_details(&self, uuid: &Uuid) -> Option<TaskDetails> {
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid);
        if task_state.is_none() {
            return None;
        }
        let task_state = task_state.unwrap();
        let details = task_state.details.read().await;
        Some(details.clone())
    }

    pub async fn start_process(&self, cmd: &str, args: &[&str]) -> Uuid {
        let details = Arc::new(RwLock::new(TaskDetails::default()));

        let cmd = cmd.to_string();
        let args = args.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let id = details.read().await.id;

        {
            let mut details = details.write().await;
            details.command = cmd.to_string();
            details.state = State::Running;
        }

        let handle = {
            let details = details.clone();
            let handle = tokio::task::spawn(async move {
                let details = details.clone();
                let mut command = Command::new(cmd);
                command.args(args);

                match command.output().await {
                    Ok(output) => {
                        let mut details = details.write().await;
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        details.output = stdout.to_string();
                        details.state = State::Finished;
                        details.finish_time = Some(SystemTime::now());
                    }
                    Err(_) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(SystemTime::now());
                        details.state = State::Failed;
                    }
                }
            });
            handle
        };
        {
            let details = details.clone();
            self.processes.write().await.insert(
                id,
                TaskState {
                    handle: Some(handle),
                    details: details.clone(),
                },
            );
        }

        id
    }

    #[instrument]
    pub async fn run_cleanup_task(&self) {
        let processes = self.processes.clone();
        let mut processes_lock = processes.write().await;
        processes_lock.retain(|_, state| {
            if let Some(handle) = &state.handle {
                if handle.is_finished() {
                    state.handle.as_ref().unwrap().abort();
                    return false;
                }
            }
            true
        });
    }
}
