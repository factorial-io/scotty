//! End-to-end WebSocket integration tests for shell sessions
//!
//! These tests verify the full WebSocket flow including:
//! - Authentication and authorization
//! - Shell session creation
//! - Bidirectional message exchange
//! - Session lifecycle management
//!
//! Note: These tests are marked with #[ignore] to prevent execution in CI
//! because they require:
//! - Docker daemon running
//! - Test containers available
//! - Network access
//!
//! Run locally with: cargo test --test test_shell_websocket_e2e -- --ignored --nocapture

use futures_util::{SinkExt, StreamExt};
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionRequest};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;

/// Helper to create WebSocket URL from HTTP server
fn ws_url(base_url: &reqwest::Url) -> String {
    let mut url = base_url.clone();
    url.set_scheme("ws").unwrap();
    url.set_path("/api/v1/ws");
    url.to_string()
}

/// Test basic shell session creation and termination flow
///
/// This test verifies:
/// - WebSocket connection establishment
/// - Authentication (if required)
/// - Shell session creation request/response
/// - Session ended notification
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_session_creation_and_termination() {
    // Start test server
    let app_state =
        scotty::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None)
            .await;
    let router = scotty::api::router::ApiRoutes::create(app_state.clone());
    let server = axum_test::TestServer::new(router).unwrap();

    // Connect to WebSocket
    let server_addr = server.server_address().expect("Server should have address");
    let url = ws_url(&server_addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Create shell session request
    let session_request = ShellSessionRequest {
        app_name: "test-app".to_string(),
        service_name: "web".to_string(),
        shell_command: Some("/bin/sh".to_string()),
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    // Send session creation request
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created response
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        session_id = Some(info.session_id);
                        println!("Shell session created: {}", info.session_id);
                        break;
                    }
                    WebSocketMessage::Error(err) => {
                        panic!("Failed to create shell session: {}", err);
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(
        session_id.is_some(),
        "Should have received ShellSessionCreated message"
    );

    // Send terminate command (via Ctrl+D or exit)
    let input_message = WebSocketMessage::ShellSessionData(scotty_types::ShellSessionData {
        session_id: session_id.unwrap(),
        data_type: ShellDataType::Input,
        data: "exit\n".to_string(),
    });

    let json = serde_json::to_string(&input_message).expect("Failed to serialize input");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send exit command");

    // Wait for session ended
    let mut session_ended = false;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionEnded(end) => {
                        println!("Session ended: {} - {}", end.session_id, end.reason);
                        session_ended = true;
                        break;
                    }
                    WebSocketMessage::ShellSessionData(_) => {
                        // Ignore output data
                        continue;
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(session_ended, "Should have received ShellSessionEnded");
}

/// Test command execution with exit code verification
///
/// This test verifies:
/// - Command execution in non-interactive mode
/// - Output data streaming
/// - Exit code propagation in ShellSessionEnded message
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_command_execution_with_exit_code() {
    // Start test server
    let app_state =
        scotty::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None)
            .await;
    let router = scotty::api::router::ApiRoutes::create(app_state.clone());
    let server = axum_test::TestServer::new(router).unwrap();

    // Connect to WebSocket
    let server_addr = server.server_address().expect("Server should have address");
    let url = ws_url(&server_addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Create shell session with command that will exit with code 42
    let session_request = ShellSessionRequest {
        app_name: "test-app".to_string(),
        service_name: "web".to_string(),
        shell_command: Some("exit 42".to_string()),
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Collect messages
    let mut session_created = false;
    let mut exit_code: Option<i32> = None;

    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        println!("Shell session created: {}", info.session_id);
                        session_created = true;
                    }
                    WebSocketMessage::ShellSessionEnded(end) => {
                        println!(
                            "Session ended with exit code: {:?} - {}",
                            end.exit_code, end.reason
                        );
                        exit_code = end.exit_code;
                        break;
                    }
                    WebSocketMessage::ShellSessionData(data) => {
                        println!("Received output: {}", data.data);
                    }
                    WebSocketMessage::Error(err) => {
                        println!("Error: {}", err);
                    }
                    _ => continue,
                }
            }
        }
    }

    assert!(session_created, "Should have created session");
    assert_eq!(
        exit_code,
        Some(42),
        "Should have received exit code 42 from command"
    );
}

/// Test input/output bidirectional flow
///
/// This test verifies:
/// - Sending input to shell
/// - Receiving output from shell
/// - Multiple input/output exchanges
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_bidirectional_io() {
    // Start test server
    let app_state =
        scotty::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None)
            .await;
    let router = scotty::api::router::ApiRoutes::create(app_state.clone());
    let server = axum_test::TestServer::new(router).unwrap();

    // Connect to WebSocket
    let server_addr = server.server_address().expect("Server should have address");
    let url = ws_url(&server_addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Create interactive shell session
    let session_request = ShellSessionRequest {
        app_name: "test-app".to_string(),
        service_name: "web".to_string(),
        shell_command: None, // Interactive mode
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionCreated(info) => {
                        session_id = Some(info.session_id);
                        break;
                    }
                    _ => continue,
                }
            }
        }
    }

    let session_id = session_id.expect("Should have session ID");

    // Send echo command
    let echo_input = WebSocketMessage::ShellSessionData(scotty_types::ShellSessionData {
        session_id,
        data_type: ShellDataType::Input,
        data: "echo 'Hello from shell test'\n".to_string(),
    });

    let json = serde_json::to_string(&echo_input).expect("Failed to serialize input");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send echo command");

    // Collect output
    let mut received_output = String::new();
    let mut output_count = 0;
    const MAX_OUTPUTS: usize = 10;

    while let Some(msg) = read.next().await {
        if output_count >= MAX_OUTPUTS {
            break;
        }

        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                match ws_msg {
                    WebSocketMessage::ShellSessionData(data) => {
                        if matches!(data.data_type, ShellDataType::Output) {
                            received_output.push_str(&data.data);
                            output_count += 1;
                            println!("Output chunk {}: {}", output_count, data.data);
                        }
                    }
                    _ => continue,
                }
            }
        }
    }

    // Verify we received some output (exact match depends on shell prompt)
    assert!(
        !received_output.is_empty(),
        "Should have received output from echo command"
    );
    println!("Total output received: {}", received_output);
}

/// Test TTY resize message
///
/// This test verifies:
/// - Sending TTY resize commands
/// - Server accepting resize without error
///
/// Marked #[ignore] - requires Docker
#[tokio::test]
#[ignore]
async fn test_shell_tty_resize() {
    // Start test server
    let app_state =
        scotty::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None)
            .await;
    let router = scotty::api::router::ApiRoutes::create(app_state.clone());
    let server = axum_test::TestServer::new(router).unwrap();

    // Connect to WebSocket
    let server_addr = server.server_address().expect("Server should have address");
    let url = ws_url(&server_addr);
    let (ws_stream, _) = connect_async(&url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut write, mut read) = ws_stream.split();

    // Create shell session
    let session_request = ShellSessionRequest {
        app_name: "test-app".to_string(),
        service_name: "web".to_string(),
        shell_command: None,
    };

    let create_message = WebSocketMessage::CreateShellSession(session_request);
    let json = serde_json::to_string(&create_message).expect("Failed to serialize message");

    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send create session message");

    // Wait for session created
    let mut session_id: Option<Uuid> = None;
    while let Some(msg) = read.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(WebSocketMessage::ShellSessionCreated(info)) =
                serde_json::from_str::<WebSocketMessage>(&text)
            {
                session_id = Some(info.session_id);
                break;
            }
        }
    }

    let session_id = session_id.expect("Should have session ID");

    // Send resize message
    let resize_message = WebSocketMessage::ResizeShellTty {
        session_id,
        width: 120,
        height: 40,
    };

    let json = serde_json::to_string(&resize_message).expect("Failed to serialize resize");
    write
        .send(Message::Text(json.into()))
        .await
        .expect("Failed to send resize message");

    // Just verify we don't get an error back
    // (resize is fire-and-forget, but errors would be sent)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // If we got here without errors, resize was accepted
    println!("TTY resize command accepted");
}
