#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::{collections::HashMap, sync::Arc, time::SystemTime};

use tokio::io::{AsyncBufReadExt, BufReader};

use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::instrument;
use uuid::Uuid;

use crate::tasks::task_details::{State, TaskDetails, TaskState};

#[derive(Clone, Debug)]
pub struct TaskManager {
    processes: Arc<RwLock<HashMap<Uuid, TaskState>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_task_details(&self, uuid: &Uuid) -> Option<TaskDetails> {
        let processes = self.processes.read().await;
        let task_state = processes.get(uuid);
        task_state?;
        let task_state = task_state.unwrap();
        let details = task_state.details.read().await;
        Some(details.clone())
    }

    pub async fn start_process<F, Fut>(
        &self,
        cwd: &Path,
        cmd: &str,
        args: &[&str],
        callback: F,
    ) -> Uuid
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + std::marker::Send,
    {
        let details = Arc::new(RwLock::new(TaskDetails::default()));

        let cwd = cwd.to_path_buf();
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

            tokio::task::spawn(async move {
                let details = details.clone();
                let exit_code = spawn_process(&cwd, &cmd, &args, &details).await;

                // Give the system a chance to update its state.
                callback().await;

                match exit_code {
                    Ok(0) => {
                        let mut details = details.write().await;
                        details.state = State::Finished;
                        details.finish_time = Some(SystemTime::now());
                    }
                    Ok(_) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(SystemTime::now());
                        details.state = State::Failed;
                    }
                    Err(e) => {
                        let mut details = details.write().await;
                        details.finish_time = Some(SystemTime::now());
                        details.state = State::Failed;
                        details.output = e.to_string();
                    }
                }
            })
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

async fn spawn_process(
    cwd: &PathBuf,
    cmd: &str,
    args: &Vec<String>,
    details: &Arc<RwLock<TaskDetails>>,
) -> anyhow::Result<i32> {
    let mut child = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start command");

    // Ensure we can get stdout
    let stdout = child.stdout.take().expect("Failed to get stdout");

    // Wrap stdout in a BufReader
    let mut reader = BufReader::new(stdout).lines();

    let mut output = String::new();
    // Stream the stdout into the variable
    while let Some(line) = reader.next_line().await? {
        output.push_str(&line);
        output.push('\n');
        {
            let mut details = details.write().await;
            details.output = output.to_string();
        }
    }
    // Wait for the command to complete
    let status = child.wait().await?;

    let exit_code = status.code().unwrap_or_default();
    Ok(exit_code)
}
