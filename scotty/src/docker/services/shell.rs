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
    pub client_id: Uuid, // WebSocket client that owns this session
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

/// Guard to ensure session cleanup even on panic
///
/// This guard uses RAII to guarantee that a session is removed from the active
/// sessions map when it goes out of scope, even if the task panics.
struct SessionGuard {
    session_id: Uuid,
    sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>,
}

impl SessionGuard {
    fn new(session_id: Uuid, sessions: Arc<RwLock<HashMap<Uuid, ShellSession>>>) -> Self {
        Self {
            session_id,
            sessions,
        }
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        // Ensure session cleanup even on panic
        // We need to block on the async operation since Drop is synchronous
        let session_id = self.session_id;
        let sessions = self.sessions.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                let mut sessions = sessions.write().await;
                sessions.remove(&session_id);
                info!("SessionGuard: cleaned up session {}", session_id);
            })
        });
    }
}

/// Service for managing container shell sessions
#[derive(Debug, Clone)]
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
        client_id: Uuid, // WebSocket client ID
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
        // Always wrap the command with sh -c to properly handle shell syntax
        let exec_options = CreateExecOptions {
            attach_stdin: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            cmd: Some(vec!["sh", "-c", shell_cmd.as_str()]),
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
            client_id,
            created_at: chrono::Utc::now(),
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id, session.clone());
        }

        // Record session started
        crate::metrics::shell::record_session_started();

        // Clone values we need to move into the spawned task
        let app_name_clone = app_data.name.clone();
        let service_name_clone = service_name.to_string();
        let container_id_clone = container_id.clone();
        let shell_cmd_clone = shell_cmd.clone();

        // Send session created message to client
        if let Err(e) = app_state
            .messenger
            .send_to_client(
                client_id,
                WebSocketMessage::ShellSessionCreated(ShellSessionInfo::new(
                    session_id,
                    app_name_clone.clone(),
                    service_name_clone.clone(),
                    container_id_clone.clone(),
                    shell_cmd_clone.clone(),
                )),
            )
            .await
        {
            warn!(
                "Failed to send session created message to client {}: {}",
                client_id, e
            );
            // Continue anyway - session is already created
        }

        // Start the shell session handler
        let docker = self.docker.clone();
        let app_state = app_state.clone();
        let active_sessions = self.active_sessions.clone();
        let session_ttl = self.shell_settings.session_ttl();

        crate::metrics::spawn_instrumented(async move {
            // Create guard to ensure session cleanup even on panic
            let _guard = SessionGuard::new(session_id, active_sessions.clone());

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
                    let mut status_check_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
                    status_check_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                    loop {
                        tokio::select! {
                            // Check for TTL timeout
                            _ = tokio::time::sleep_until(session_start + session_ttl) => {
                                info!("Shell session {} expired after TTL", session_id);
                                let duration_secs = session_start.elapsed().as_secs_f64();
                                crate::metrics::shell::record_session_timeout(duration_secs);
                                if let Err(e) = app_state.messenger.send_to_client(
                                    client_id,
                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                        session_id,
                                        exit_code: None,
                                        reason: "Session timeout".to_string(),
                                    }),
                                ).await {
                                    warn!("Failed to send timeout message to client {}: {}", client_id, e);
                                }
                                break;
                            }

                            // Periodically check if exec is still running (important for TTY mode)
                            _ = status_check_interval.tick() => {
                                match docker.inspect_exec(&exec_id).await {
                                    Ok(exec_info) => {
                                        if let Some(running) = exec_info.running {
                                            if !running {
                                                info!("Shell session {} detected exec completion via status check", session_id);
                                                let duration_secs = session_start.elapsed().as_secs_f64();
                                                crate::metrics::shell::record_session_ended(duration_secs);
                                                let exit_code = exec_info.exit_code.map(|c| c as i32);
                                                if let Err(e) = app_state.messenger.send_to_client(
                                                    client_id,
                                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                        session_id,
                                                        exit_code,
                                                        reason: "Shell exited".to_string(),
                                                    }),
                                                ).await {
                                                    warn!("Failed to send session end message to client {}: {}", client_id, e);
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to inspect exec {} for session {}: {}", exec_id, session_id, e);
                                    }
                                }
                            }

                            // Handle input commands
                            Some(cmd) = rx.recv() => {
                                match cmd {
                                    ShellCommand::Input(data) => {
                                        if let Err(e) = input.write_all(data.as_bytes()).await {
                                            error!("Failed to write to shell {}: {}", session_id, e);
                                            let duration_secs = session_start.elapsed().as_secs_f64();
                                            crate::metrics::shell::record_session_error(duration_secs);
                                            if let Err(send_err) = app_state.messenger.send_to_client(
                                                client_id,
                                                WebSocketMessage::ShellSessionError(ShellSessionError {
                                                    session_id,
                                                    error: format!("Failed to write input: {}", e),
                                                }),
                                            ).await {
                                                warn!("Failed to send error message to client {}: {}", client_id, send_err);
                                            }
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
                                        let duration_secs = session_start.elapsed().as_secs_f64();
                                        crate::metrics::shell::record_session_ended(duration_secs);
                                        if let Err(e) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                                session_id,
                                                exit_code: None,
                                                reason: "Terminated by user".to_string(),
                                            }),
                                        ).await {
                                            warn!("Failed to send termination message to client {}: {}", client_id, e);
                                        }
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
                                            LogOutput::Console { message } => {
                                                // TTY mode sends output as Console
                                                String::from_utf8_lossy(&message).to_string()
                                            }
                                            LogOutput::StdIn { .. } => continue, // Skip stdin
                                        };

                                        // If we can't send output, client is gone - terminate session
                                        if let Err(e) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionData(ShellSessionData {
                                                session_id,
                                                data_type: ShellDataType::Output,
                                                data: output_data,
                                            }),
                                        ).await {
                                            warn!("Failed to send output to client {}: {} - terminating session", client_id, e);
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to read from shell stream {}: {}", session_id, e);
                                        let duration_secs = session_start.elapsed().as_secs_f64();
                                        crate::metrics::shell::record_session_error(duration_secs);
                                        if let Err(send_err) = app_state.messenger.send_to_client(
                                            client_id,
                                            WebSocketMessage::ShellSessionError(ShellSessionError {
                                                session_id,
                                                error: format!("Failed to read output: {}", e),
                                            }),
                                        ).await {
                                            warn!("Failed to send error message to client {}: {}", client_id, send_err);
                                        }
                                        break;
                                    }
                                }
                            }
                            // Stream ended
                            else => {
                                info!("Shell session {} ended (stream closed)", session_id);
                                let duration_secs = session_start.elapsed().as_secs_f64();
                                crate::metrics::shell::record_session_ended(duration_secs);
                                if let Err(e) = app_state.messenger.send_to_client(
                                    client_id,
                                    WebSocketMessage::ShellSessionEnded(ShellSessionEnd {
                                        session_id,
                                        exit_code: Some(0),
                                        reason: "Shell exited".to_string(),
                                    }),
                                ).await {
                                    warn!("Failed to send session end message to client {}: {}", client_id, e);
                                }
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
                    crate::metrics::shell::record_session_error(0.0);
                    if let Err(e) = app_state
                        .messenger
                        .send_to_client(
                            client_id,
                            WebSocketMessage::ShellSessionError(ShellSessionError {
                                session_id,
                                error: "Shell started in detached mode".to_string(),
                            }),
                        )
                        .await
                    {
                        warn!("Failed to send detached error message to client {}: {}", client_id, e);
                    }
                }
            }

            // Session cleanup is handled automatically by SessionGuard on drop
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
