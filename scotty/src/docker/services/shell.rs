use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use bollard::Docker;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use futures_util::StreamExt;
use tracing::{info, error, warn};
use tokio::io::AsyncWriteExt;

use crate::api::message::{WebSocketMessage, ShellSessionInfo, ShellSessionData, ShellDataType, ShellSessionEnd, ShellSessionError};
use crate::api::ws::broadcast_message;
use crate::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::settings::shell::ShellSettings;

/// Active shell sessions tracked by the service
#[derive(Debug, Clone)]
pub struct ShellSession {
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub exec_id: String,
    pub sender: mpsc::Sender<ShellCommand>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Commands that can be sent to a shell session
#[derive(Debug)]
pub enum ShellCommand {
    Input(String),
    Resize { width: u16, height: u16 },
    Terminate,
}

/// Service for managing container shell sessions
#[derive(Clone)]
pub struct ShellService {
    docker: Docker,
    active_sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
    shell_settings: ShellSettings,
}

impl ShellService {
    pub fn new(docker: Docker, shell_settings: ShellSettings) -> Self {
        Self {
            docker,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            shell_settings,
        }
    }

    /// Create a new shell session
    pub async fn create_session(
        &self,
        app_state: &SharedAppState,
        app_data: &AppData,
        service_name: &str,
        shell_command: Option<String>,
    ) -> Result<Uuid, String> {
        // Check session limits
        {
            let sessions = self.active_sessions.read().await;
            let app_sessions = sessions.values()
                .filter(|s| s.app_name == app_data.name)
                .count();

            if app_sessions >= self.shell_settings.max_sessions_per_app {
                return Err(format!(
                    "Maximum sessions per app ({}) reached",
                    self.shell_settings.max_sessions_per_app
                ));
            }

            if sessions.len() >= self.shell_settings.max_sessions_global {
                return Err(format!(
                    "Maximum global sessions ({}) reached",
                    self.shell_settings.max_sessions_global
                ));
            }
        }

        // Find the container for the service
        let container_state = app_data.services
            .iter()
            .find(|s| s.service == service_name)
            .ok_or_else(|| format!("Service '{}' not found in app '{}'", service_name, app_data.name))?;

        let container_id = container_state.id
            .as_ref()
            .ok_or_else(|| format!("Service '{}' has no container ID", service_name))?;

        // Generate session ID
        let session_id = Uuid::new_v4();

        // Determine shell command to use
        let shell_cmd = shell_command
            .unwrap_or_else(|| self.shell_settings.default_shell.clone());

        // Prepare environment variables
        let env_vars: Vec<String> = self.shell_settings.default_env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let env_refs: Vec<&str> = env_vars.iter().map(|s| s.as_str()).collect();

        // Create exec instance
        let exec_options = CreateExecOptions {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            cmd: Some(vec![shell_cmd.as_str()]),
            env: if env_refs.is_empty() { None } else { Some(env_refs) },
            ..Default::default()
        };

        let exec_creation = self.docker
            .create_exec(container_id, exec_options)
            .await
            .map_err(|e| format!("Failed to create exec: {}", e))?;

        let exec_id = exec_creation.id;

        // Start exec instance
        let start_options = StartExecOptions {
            detach: false,
            tty: true,
            ..Default::default()
        };

        let exec_stream = self.docker
            .start_exec(&exec_id, Some(start_options))
            .await
            .map_err(|e| format!("Failed to start exec: {}", e))?;

        let (tx, mut rx) = mpsc::channel::<ShellCommand>(100);

        // Create session info
        let session = ShellSession {
            session_id,
            app_name: app_data.name.clone(),
            service_name: service_name.to_string(),
            container_id: container_id.clone(),
            exec_id: exec_id.clone(),
            sender: tx,
            created_at: chrono::Utc::now(),
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, session.clone());
        }

        // Clone values we need to move into the spawned task
        let app_name_clone = app_data.name.clone();
        let service_name_clone = service_name.to_string();
        let container_id_clone = container_id.clone();
        let shell_cmd_clone = shell_cmd.clone();

        // Send session created message
        broadcast_message(
            app_state,
            WebSocketMessage::ShellSessionCreated(ShellSessionInfo {
                session_id,
                app_name: app_name_clone.clone(),
                service_name: service_name_clone.clone(),
                container_id: container_id_clone.clone(),
                shell_command: shell_cmd_clone.clone(),
            }),
        ).await;

        // Start the shell session handler
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_sessions = self.active_sessions.clone();
        let session_ttl = self.shell_settings.session_ttl();

        tokio::spawn(async move {
            info!("Starting shell session {} for container {}", session_id, container_id_clone);

            match exec_stream {
                StartExecResults::Attached { mut input, mut output } => {
                    let session_start = tokio::time::Instant::now();

                    loop {
                        tokio::select! {
                            // Check for TTL timeout
                            _ = tokio::time::sleep_until(session_start + session_ttl) => {
                                info!("Shell session {} expired after TTL", session_id);
                                broadcast_message(
                                    &app_state,
                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                        session_id,
                                        exit_code: None,
                                        reason: "Session timeout".to_string(),
                                    }),
                                ).await;
                                break;
                            }

                            // Handle input commands
                            Some(cmd) = rx.recv() => {
                                match cmd {
                                    ShellCommand::Input(data) => {
                                        if let Err(e) = input.write_all(data.as_bytes()).await {
                                            error!("Failed to write to shell {}: {}", session_id, e);
                                            broadcast_message(
                                                &app_state,
                                                WebSocketMessage::ShellSessionError(ShellSessionError {
                                                    session_id,
                                                    error: format!("Failed to write input: {}", e),
                                                }),
                                            ).await;
                                            break;
                                        }
                                        if let Err(e) = input.flush().await {
                                            error!("Failed to flush shell input {}: {}", session_id, e);
                                        }
                                    }
                                    ShellCommand::Resize { width, height } => {
                                        // Resize the TTY
                                        if let Err(e) = docker.resize_exec(&exec_id, bollard::exec::ResizeExecOptions {
                                            height,
                                            width,
                                        }).await {
                                            warn!("Failed to resize TTY for session {}: {}", session_id, e);
                                        }
                                    }
                                    ShellCommand::Terminate => {
                                        info!("Terminating shell session {} by request", session_id);
                                        broadcast_message(
                                            &app_state,
                                            WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                session_id,
                                                exit_code: None,
                                                reason: "Terminated by user".to_string(),
                                            }),
                                        ).await;
                                        break;
                                    }
                                }
                            }

                            // Read output from shell stream
                            Some(result) = output.next() => {
                                match result {
                                    Ok(log_output) => {
                                        use bollard::container::LogOutput;
                                        let output_data = match log_output {
                                            LogOutput::StdOut { message } => {
                                                String::from_utf8_lossy(&message).to_string()
                                            }
                                            LogOutput::StdErr { message } => {
                                                String::from_utf8_lossy(&message).to_string()
                                            }
                                            LogOutput::StdIn { .. } => continue, // Skip stdin
                                            LogOutput::Console { .. } => continue, // Skip console
                                        };

                                        broadcast_message(
                                            &app_state,
                                            WebSocketMessage::ShellSessionData(ShellSessionData {
                                                session_id,
                                                data_type: ShellDataType::Output,
                                                data: output_data,
                                            }),
                                        ).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to read from shell stream {}: {}", session_id, e);
                                        broadcast_message(
                                            &app_state,
                                            WebSocketMessage::ShellSessionError(ShellSessionError {
                                                session_id,
                                                error: format!("Failed to read output: {}", e),
                                            }),
                                        ).await;
                                        break;
                                    }
                                }
                            }
                            // Stream ended
                            else => {
                                info!("Shell session {} ended (stream closed)", session_id);
                                broadcast_message(
                                    &app_state,
                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                        session_id,
                                        exit_code: Some(0),
                                        reason: "Shell exited".to_string(),
                                    }),
                                ).await;
                                break;
                            }
                        }
                    }
                }
                StartExecResults::Detached => {
                    error!("Shell session {} started in detached mode (unexpected)", session_id);
                    broadcast_message(
                        &app_state,
                        WebSocketMessage::ShellSessionError(ShellSessionError {
                            session_id,
                            error: "Shell started in detached mode".to_string(),
                        }),
                    ).await;
                }
            }

            // Clean up session
            {
                let mut sessions = active_sessions.write().await;
                sessions.remove(&session_id);
            }

            info!("Shell session {} cleaned up", session_id);
        });

        Ok(session_id)
    }

    /// Send input to a shell session
    pub async fn send_input(&self, session_id: Uuid, input: String) -> Result<(), String> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session.sender.send(ShellCommand::Input(input)).await
                .map_err(|_| "Failed to send input command".to_string())?;
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }

    /// Resize a shell session TTY
    pub async fn resize_tty(&self, session_id: Uuid, width: u16, height: u16) -> Result<(), String> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session.sender.send(ShellCommand::Resize { width, height }).await
                .map_err(|_| "Failed to send resize command".to_string())?;
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }

    /// Terminate a shell session
    pub async fn terminate_session(&self, session_id: Uuid) -> Result<(), String> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session.sender.send(ShellCommand::Terminate).await
                .map_err(|_| "Failed to send terminate command".to_string())?;
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }

    /// Get active sessions
    pub async fn get_active_sessions(&self) -> Vec<ShellSession> {
        let sessions = self.active_sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Terminate all sessions for an app
    pub async fn terminate_app_sessions(&self, app_name: &str) {
        let sessions = self.active_sessions.read().await;
        let app_sessions: Vec<_> = sessions.values()
            .filter(|s| s.app_name == app_name)
            .cloned()
            .collect();
        drop(sessions);

        for session in app_sessions {
            let _ = session.sender.send(ShellCommand::Terminate).await;
        }
    }
}