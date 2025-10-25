use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::app_state::SharedAppState;
use scotty_core::apps::app_data::AppData;
use scotty_core::settings::shell::ShellSettings;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{
    ShellDataType, ShellSessionData, ShellSessionEnd, ShellSessionError, ShellSessionInfo,
};

use thiserror::Error;

/// Error types for shell session operations
#[derive(Error, Debug, Clone, utoipa::ToSchema)]
pub enum ShellServiceError {
    #[error("Service '{service}' not found in app '{app}'")]
    ServiceNotFound { service: String, app: String },

    #[error("Service '{service}' has no container ID")]
    NoContainerId { service: String },

    #[error("Session '{session_id}' not found")]
    SessionNotFound { session_id: Uuid },

    #[error("Maximum sessions per app ({limit}) reached")]
    MaxSessionsPerApp { limit: usize },

    #[error("Maximum global sessions ({limit}) reached")]
    MaxSessionsGlobal { limit: usize },

    #[error("Failed to send command to session: {reason}")]
    CommandSendFailed { reason: String },

    #[error("Docker operation failed: {operation} - {message}")]
    DockerOperationFailed { operation: String, message: String },
}

/// Result type alias for shell service operations
pub type ShellServiceResult<T> = Result<T, ShellServiceError>;

/// Active shell sessions tracked by the service
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ShellSession {
    pub session_id: Uuid,
    pub app_name: String,
    pub service_name: String,
    pub container_id: String,
    pub exec_id: String,
    pub sender: mpsc::Sender<ShellCommand>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl ShellSession {
    /// Check if session has expired based on TTL
    #[allow(dead_code)]
    pub fn is_expired(&self, ttl: std::time::Duration) -> bool {
        let age = chrono::Utc::now() - self.created_at;
        age.num_seconds() as u64 > ttl.as_secs()
    }

    /// Convert to ShellSessionInfo for WebSocket messages
    #[allow(dead_code)]
    pub fn to_info(&self, shell_command: String) -> ShellSessionInfo {
        ShellSessionInfo::new(
            self.session_id,
            self.app_name.clone(),
            self.service_name.clone(),
            self.container_id.clone(),
            shell_command,
        )
    }

    /// Send a command to this session
    pub async fn send_command(&self, cmd: ShellCommand) -> ShellServiceResult<()> {
        self.sender
            .send(cmd)
            .await
            .map_err(|e| ShellServiceError::CommandSendFailed {
                reason: e.to_string(),
            })
    }
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
    ) -> ShellServiceResult<Uuid> {
        // Check session limits
        {
            let sessions = self.active_sessions.read().await;
            let app_sessions = sessions
                .values()
                .filter(|s| s.app_name == app_data.name)
                .count();

            if app_sessions >= self.shell_settings.max_sessions_per_app {
                return Err(ShellServiceError::MaxSessionsPerApp {
                    limit: self.shell_settings.max_sessions_per_app,
                });
            }

            if sessions.len() >= self.shell_settings.max_sessions_global {
                return Err(ShellServiceError::MaxSessionsGlobal {
                    limit: self.shell_settings.max_sessions_global,
                });
            }
        }

        // Find the container for the service
        let container_id = app_data
            .get_container_id_for_service(service_name)
            .ok_or_else(|| {
                if app_data.find_container_by_service(service_name).is_some() {
                    ShellServiceError::NoContainerId {
                        service: service_name.to_string(),
                    }
                } else {
                    ShellServiceError::ServiceNotFound {
                        service: service_name.to_string(),
                        app: app_data.name.clone(),
                    }
                }
            })?;

        // Generate session ID
        let session_id = Uuid::new_v4();

        // Determine shell command to use
        let shell_cmd = shell_command.unwrap_or_else(|| self.shell_settings.default_shell.clone());

        // Prepare environment variables
        let env_vars: Vec<String> = self
            .shell_settings
            .default_env
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
            env: if env_refs.is_empty() {
                None
            } else {
                Some(env_refs)
            },
            ..Default::default()
        };

        let exec_creation = self
            .docker
            .create_exec(container_id, exec_options)
            .await
            .map_err(|e| ShellServiceError::DockerOperationFailed {
                operation: "create exec".to_string(),
                message: e.to_string(),
            })?;

        let exec_id = exec_creation.id;

        // Start exec instance
        let start_options = StartExecOptions {
            detach: false,
            tty: true,
            ..Default::default()
        };

        let exec_stream = self
            .docker
            .start_exec(&exec_id, Some(start_options))
            .await
            .map_err(|e| ShellServiceError::DockerOperationFailed {
                operation: "start exec".to_string(),
                message: e.to_string(),
            })?;

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
        app_state
            .messenger
            .broadcast_to_all(WebSocketMessage::ShellSessionCreated(
                ShellSessionInfo::new(
                    session_id,
                    app_name_clone.clone(),
                    service_name_clone.clone(),
                    container_id_clone.clone(),
                    shell_cmd_clone.clone(),
                ),
            ))
            .await;

        // Start the shell session handler
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_sessions = self.active_sessions.clone();
        let session_ttl = self.shell_settings.session_ttl();

        crate::metrics::spawn_instrumented(async move {
            info!(
                "Starting shell session {} for container {}",
                session_id, container_id_clone
            );

            match exec_stream {
                StartExecResults::Attached {
                    mut input,
                    mut output,
                } => {
                    let session_start = tokio::time::Instant::now();

                    loop {
                        tokio::select! {
                            // Check for TTL timeout
                            _ = tokio::time::sleep_until(session_start + session_ttl) => {
                                info!("Shell session {} expired after TTL", session_id);
                                app_state.messenger.broadcast_to_all(
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
                                            app_state.messenger.broadcast_to_all(
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
                                        app_state.messenger.broadcast_to_all(
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

                                        app_state.messenger.broadcast_to_all(
                                            WebSocketMessage::ShellSessionData(ShellSessionData {
                                                session_id,
                                                data_type: ShellDataType::Output,
                                                data: output_data,
                                            }),
                                        ).await;
                                    }
                                    Err(e) => {
                                        error!("Failed to read from shell stream {}: {}", session_id, e);
                                        app_state.messenger.broadcast_to_all(
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
                                app_state.messenger.broadcast_to_all(
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
                    error!(
                        "Shell session {} started in detached mode (unexpected)",
                        session_id
                    );
                    app_state
                        .messenger
                        .broadcast_to_all(WebSocketMessage::ShellSessionError(ShellSessionError {
                            session_id,
                            error: "Shell started in detached mode".to_string(),
                        }))
                        .await;
                }
            }

            // Clean up session
            {
                let mut sessions = active_sessions.write().await;
                sessions.remove(&session_id);
            }

            info!("Shell session {} cleaned up", session_id);
        }).await;

        Ok(session_id)
    }

    /// Send input to a shell session
    pub async fn send_input(&self, session_id: Uuid, input: String) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session.send_command(ShellCommand::Input(input)).await
        } else {
            Err(ShellServiceError::SessionNotFound { session_id })
        }
    }

    /// Resize a shell session TTY
    pub async fn resize_tty(
        &self,
        session_id: Uuid,
        width: u16,
        height: u16,
    ) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session
                .send_command(ShellCommand::Resize { width, height })
                .await
        } else {
            Err(ShellServiceError::SessionNotFound { session_id })
        }
    }

    /// Terminate a shell session
    pub async fn terminate_session(&self, session_id: Uuid) -> ShellServiceResult<()> {
        let sessions = self.active_sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            session.send_command(ShellCommand::Terminate).await
        } else {
            Err(ShellServiceError::SessionNotFound { session_id })
        }
    }

    /// Get active sessions
    #[allow(dead_code)]
    pub async fn get_active_sessions(&self) -> Vec<ShellSession> {
        let sessions = self.active_sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Terminate all sessions for an app
    #[allow(dead_code)]
    pub async fn terminate_app_sessions(&self, app_name: &str) {
        let sessions = self.active_sessions.read().await;
        let app_sessions: Vec<_> = sessions
            .values()
            .filter(|s| s.app_name == app_name)
            .cloned()
            .collect();
        drop(sessions);

        for session in app_sessions {
            let _ = session.sender.send(ShellCommand::Terminate).await;
        }
    }
}
