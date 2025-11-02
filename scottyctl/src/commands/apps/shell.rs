use anyhow::Context;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use futures_util::StreamExt;
use std::io::{stdout, Write};
use tokio::select;
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error};

use crate::{cli::ShellCommand, context::AppContext};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionData};
use uuid::Uuid;

/// Open an interactive shell for an app service
pub async fn shell_app(context: &AppContext, cmd: &ShellCommand) -> anyhow::Result<()> {
    // Validate app and service using shared utility
    let _app_data = super::validate_app_and_service(
        context,
        &cmd.app_name,
        &cmd.service_name,
        "app:shell",
    )
    .await?;

    // Create shell session and open interactive terminal
    open_shell(context, cmd).await
}

/// Create shell session and open interactive terminal
async fn open_shell(context: &AppContext, cmd: &ShellCommand) -> anyhow::Result<()> {
    use crate::websocket::AuthenticatedWebSocket;
    use scotty_types::ShellSessionRequest;

    let ui = context.ui();

    // Connect to WebSocket first
    ui.new_status_line("Connecting to WebSocket...");
    let mut ws = AuthenticatedWebSocket::connect(context.server())
        .await
        .context("Failed to connect to WebSocket")?;

    ui.success("🔐 WebSocket authenticated");

    // Build shell command: either custom shell, or bash -c "command", or just bash
    let shell_command = if let Some(command) = &cmd.command {
        let shell = cmd.shell.as_deref().unwrap_or("/bin/bash");
        Some(format!("{} -c '{}'", shell, command.replace('\'', "'\\''")))
    } else {
        cmd.shell.clone()
    };

    // Create shell session via WebSocket
    ui.new_status_line("Creating shell session...");

    let request = ShellSessionRequest {
        app_name: cmd.app_name.clone(),
        service_name: cmd.service_name.clone(),
        shell_command,
    };

    // Send CreateShellSession message
    let message = WebSocketMessage::CreateShellSession(request);
    ws.send(message).await
        .context("Failed to send shell session creation request")?;

    // Wait for ShellSessionCreated response
    use futures_util::StreamExt;
    let session_id = loop {
        match ws.receiver.next().await {
            Some(Ok(Message::Text(text))) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::ShellSessionCreated(info) => {
                            ui.success(format!("Shell session created: {}", info.session_id));
                            break info.session_id;
                        }
                        WebSocketMessage::Error(err) => {
                            anyhow::bail!("Failed to create shell session: {}", err);
                        }
                        _ => {
                            // Ignore other messages while waiting for session creation
                            continue;
                        }
                    }
                }
            }
            Some(Ok(Message::Close(_))) => {
                anyhow::bail!("WebSocket closed while waiting for session creation");
            }
            Some(Err(e)) => {
                anyhow::bail!("WebSocket error: {}", e);
            }
            None => {
                anyhow::bail!("WebSocket stream ended");
            }
            _ => continue,
        }
    };

    // If running a single command, just wait for output and exit
    if cmd.command.is_some() {
        return run_command_mode(ws, session_id).await;
    }

    // Otherwise, run interactive mode
    run_interactive_mode(ws, session_id, ui).await
}

/// Run a single command and exit (non-interactive mode)
async fn run_command_mode(mut ws: crate::websocket::AuthenticatedWebSocket, _session_id: Uuid) -> anyhow::Result<()> {
    // Just listen for output and print it
    while let Some(message) = ws.receiver.next().await {
        match message {
            Ok(Message::Text(text)) => {
                if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                    match ws_message {
                        WebSocketMessage::ShellSessionData(data) => {
                            // Print shell output directly
                            print!("{}", data.data);
                            let _ = stdout().flush();
                        }
                        WebSocketMessage::ShellSessionEnded(_end) => {
                            break;
                        }
                        WebSocketMessage::ShellSessionError(error) => {
                            eprintln!("Shell error: {}", error.error);
                            break;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    let _ = ws.close().await;
    Ok(())
}

/// Run interactive shell mode with raw terminal
async fn run_interactive_mode(
    mut ws: crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
    ui: &crate::utils::ui::Ui,
) -> anyhow::Result<()> {
    // Clear the UI status line before entering raw mode
    ui.clear();

    // Enable raw mode for terminal
    enable_raw_mode().context("Failed to enable raw mode")?;

    // Ensure we disable raw mode on exit
    let result = run_interactive_loop(&mut ws, session_id).await;

    disable_raw_mode().context("Failed to disable raw mode")?;

    // Close WebSocket
    let _ = ws.close().await;

    result
}

/// Main interactive loop handling bidirectional I/O
async fn run_interactive_loop(
    ws: &mut crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
) -> anyhow::Result<()> {
    let mut event_stream = EventStream::new();
    let mut should_exit = false;

    while !should_exit {
        select! {
            // Handle keyboard events
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(event)) => {
                        if let Some(exit) = handle_terminal_event(event, ws, session_id).await? {
                            should_exit = exit;
                        }
                    }
                    Some(Err(e)) => {
                        error!("Error reading terminal event: {}", e);
                        break;
                    }
                    None => break,
                }
            }

            // Handle WebSocket messages
            maybe_message = ws.receiver.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                            if let Some(exit) = handle_websocket_message(ws_message).await? {
                                should_exit = exit;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("WebSocket closed by server");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Handle terminal input events
async fn handle_terminal_event(
    event: Event,
    ws: &mut crate::websocket::AuthenticatedWebSocket,
    session_id: Uuid,
) -> anyhow::Result<Option<bool>> {
    match event {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Ctrl+C - send interrupt to shell
            debug!("Ctrl+C pressed");
            let input_data = ShellSessionData {
                session_id,
                data_type: ShellDataType::Input,
                data: "\x03".to_string(), // Send Ctrl+C as raw byte
            };
            let message = WebSocketMessage::ShellSessionData(input_data);
            ws.send(message).await?;
            Ok(None)
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Ctrl+D - exit interactive mode
            debug!("Ctrl+D pressed, exiting");
            Ok(Some(true))
        }
        Event::Key(KeyEvent { code, modifiers, .. }) => {
            // Convert key event to string and send to shell
            if let Some(input) = key_to_string(code, modifiers) {
                let input_data = ShellSessionData {
                    session_id,
                    data_type: ShellDataType::Input,
                    data: input,
                };
                let message = WebSocketMessage::ShellSessionData(input_data);
                ws.send(message).await?;
            }
            Ok(None)
        }
        Event::Resize(width, height) => {
            // Send terminal resize event
            debug!("Terminal resized to {}x{}", width, height);
            // TODO: Send resize message to server
            // For now, we'll skip this as it requires additional WebSocket message types
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Handle WebSocket messages from server
async fn handle_websocket_message(message: WebSocketMessage) -> anyhow::Result<Option<bool>> {
    match message {
        WebSocketMessage::ShellSessionData(data) => {
            // Print shell output directly to stdout
            print!("{}", data.data);
            let _ = stdout().flush();
            Ok(None)
        }
        WebSocketMessage::ShellSessionEnded(end) => {
            println!("\r\nShell session ended: {}", end.reason);
            Ok(Some(true))
        }
        WebSocketMessage::ShellSessionError(error) => {
            eprintln!("\r\nShell error: {}", error.error);
            Ok(Some(true))
        }
        WebSocketMessage::Error(msg) => {
            eprintln!("\r\nWebSocket error: {}", msg);
            Ok(Some(true))
        }
        _ => Ok(None),
    }
}

/// Convert crossterm key codes to string input
fn key_to_string(code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    match code {
        KeyCode::Char(c) => {
            if modifiers.contains(KeyModifiers::CONTROL) {
                // Handle other Ctrl combinations if needed
                None
            } else {
                Some(c.to_string())
            }
        }
        KeyCode::Enter => Some("\r".to_string()),
        KeyCode::Backspace => Some("\x7f".to_string()),
        KeyCode::Tab => Some("\t".to_string()),
        KeyCode::Esc => Some("\x1b".to_string()),
        KeyCode::Up => Some("\x1b[A".to_string()),
        KeyCode::Down => Some("\x1b[B".to_string()),
        KeyCode::Right => Some("\x1b[C".to_string()),
        KeyCode::Left => Some("\x1b[D".to_string()),
        KeyCode::Home => Some("\x1b[H".to_string()),
        KeyCode::End => Some("\x1b[F".to_string()),
        KeyCode::PageUp => Some("\x1b[5~".to_string()),
        KeyCode::PageDown => Some("\x1b[6~".to_string()),
        KeyCode::Delete => Some("\x1b[3~".to_string()),
        KeyCode::Insert => Some("\x1b[2~".to_string()),
        _ => None,
    }
}
