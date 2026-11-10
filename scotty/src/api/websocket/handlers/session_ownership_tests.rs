//! Session ownership tests for WebSocket messages.
//!
//! These drive the real message dispatcher with stand-in sessions, so they run
//! without a Docker daemon. They cover the scenarios of the
//! `websocket-session-authorization` spec.

use std::time::Duration;

use axum::extract::ws::Message;
use scotty_core::websocket::message::WebSocketMessage;
use scotty_types::{ShellDataType, ShellSessionData};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::handle_websocket_message;
use crate::api::auth_core::CurrentUser;
use crate::app_state::{SharedAppState, WebSocketClient};
use crate::docker::services::logs::LogStreamCommand;
use crate::docker::services::shell::ShellCommand;

async fn app_state() -> SharedAppState {
    crate::api::test_utils::create_test_app_state_with_config("tests/test_bearer_auth", None).await
}

fn user(email: &str) -> CurrentUser {
    CurrentUser {
        email: email.to_string(),
        name: email.to_string(),
        picture: None,
        access_token: None,
    }
}

/// Register a WebSocket client, optionally authenticated as `email`.
async fn connect(
    state: &SharedAppState,
    email: Option<&str>,
) -> (Uuid, broadcast::Receiver<Message>) {
    let client_id = Uuid::new_v4();
    let (tx, rx) = broadcast::channel(64);
    state
        .messenger
        .add_client(client_id, WebSocketClient::new(tx))
        .await;
    if let Some(email) = email {
        state
            .messenger
            .authenticate_client(client_id, user(email))
            .await
            .unwrap();
    }
    (client_id, rx)
}

/// Wait for the next error message sent to a client.
async fn expect_error(rx: &mut broadcast::Receiver<Message>) -> String {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("expected an error message")
            .unwrap();
        if let Message::Text(text) = msg {
            if let Ok(WebSocketMessage::Error(err)) = serde_json::from_str(&text) {
                return err;
            }
        }
    }
}

fn input(session_id: Uuid, data: &str) -> WebSocketMessage {
    WebSocketMessage::ShellSessionData(ShellSessionData::text(
        session_id,
        ShellDataType::Input,
        data,
    ))
}

#[tokio::test]
async fn unauthenticated_shell_input_is_rejected() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;
    let (anon, mut anon_rx) = connect(&state, None).await;

    handle_websocket_message(&state, anon, &input(session_id, "id\n")).await;

    assert!(expect_error(&mut anon_rx)
        .await
        .contains("Authentication required"));
    assert!(
        session_rx.try_recv().is_err(),
        "input must not reach the shell"
    );
}

#[tokio::test]
async fn owner_shell_input_is_delivered() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;

    handle_websocket_message(&state, owner, &input(session_id, "ls\n")).await;

    match session_rx.try_recv() {
        Ok(ShellCommand::Input(data)) => assert_eq!(data, b"ls\n"),
        other => panic!("expected input to reach the shell, got {other:?}"),
    }
}

#[tokio::test]
async fn owner_base64_shell_input_is_decoded() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;

    let bytes = [0x00, 0xff, 0x80, b'\n'];
    let msg = WebSocketMessage::ShellSessionData(ShellSessionData::binary(
        session_id,
        ShellDataType::Input,
        &bytes,
    ));
    handle_websocket_message(&state, owner, &msg).await;

    match session_rx.try_recv() {
        Ok(ShellCommand::Input(data)) => assert_eq!(data, bytes),
        other => panic!("expected decoded input to reach the shell, got {other:?}"),
    }
}

#[tokio::test]
async fn invalid_base64_shell_input_is_rejected() {
    let state = app_state().await;
    let (owner, mut owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;

    let mut data = ShellSessionData::text(session_id, ShellDataType::Input, "not base64!");
    data.encoding = Some(scotty_types::ShellDataEncoding::Base64);
    handle_websocket_message(&state, owner, &WebSocketMessage::ShellSessionData(data)).await;

    assert!(expect_error(&mut owner_rx).await.contains("base64"));
    assert!(
        session_rx.try_recv().is_err(),
        "input must not reach the shell"
    );
}

#[tokio::test]
async fn owner_can_close_shell_stdin() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;

    handle_websocket_message(
        &state,
        owner,
        &WebSocketMessage::CloseShellStdin { session_id },
    )
    .await;

    assert!(matches!(
        session_rx.try_recv(),
        Ok(ShellCommand::CloseStdin)
    ));
}

#[tokio::test]
async fn unauthenticated_close_shell_stdin_is_rejected() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;
    let (anon, mut anon_rx) = connect(&state, None).await;

    handle_websocket_message(
        &state,
        anon,
        &WebSocketMessage::CloseShellStdin { session_id },
    )
    .await;

    assert!(expect_error(&mut anon_rx)
        .await
        .contains("Authentication required"));
    assert!(session_rx.try_recv().is_err(), "stdin must stay open");
}

#[tokio::test]
async fn resize_of_session_without_tty_is_ignored() {
    let state = app_state().await;
    let (owner, mut owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state
        .shell_service
        .insert_test_session_without_tty("app", owner)
        .await;

    handle_websocket_message(
        &state,
        owner,
        &WebSocketMessage::ResizeShellTty {
            session_id,
            width: 80,
            height: 24,
        },
    )
    .await;

    assert!(
        session_rx.try_recv().is_err(),
        "resize must not reach the exec"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(200), owner_rx.recv())
            .await
            .is_err(),
        "resize must not produce an error"
    );
}

#[tokio::test]
async fn other_user_cannot_control_session() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;
    let (other, mut other_rx) = connect(&state, Some("other@example.com")).await;

    let messages = [
        input(session_id, "id\n"),
        WebSocketMessage::ResizeShellTty {
            session_id,
            width: 80,
            height: 24,
        },
        WebSocketMessage::CloseShellStdin { session_id },
        WebSocketMessage::TerminateShellSession { session_id },
    ];
    for msg in &messages {
        handle_websocket_message(&state, other, msg).await;
        let err = expect_error(&mut other_rx).await;
        assert!(
            err.contains("not found"),
            "error must not reveal that the session exists: {err}"
        );
    }
    assert!(session_rx.try_recv().is_err(), "session must be unaffected");
}

#[tokio::test]
async fn same_user_on_second_connection_cannot_terminate() {
    let state = app_state().await;
    let (first, _first_rx) = connect(&state, Some("owner@example.com")).await;
    let (session_id, mut session_rx) = state.shell_service.insert_test_session("app", first).await;
    let (second, mut second_rx) = connect(&state, Some("owner@example.com")).await;

    handle_websocket_message(
        &state,
        second,
        &WebSocketMessage::TerminateShellSession { session_id },
    )
    .await;

    expect_error(&mut second_rx).await;
    assert!(session_rx.try_recv().is_err(), "session must keep running");
}

#[tokio::test]
async fn unauthenticated_log_stream_stop_is_rejected() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (stream_id, mut stream_rx) = state
        .logs_service
        .insert_test_stream("app", Some(owner))
        .await;
    let (anon, mut anon_rx) = connect(&state, None).await;

    handle_websocket_message(&state, anon, &WebSocketMessage::StopLogStream { stream_id }).await;

    assert!(expect_error(&mut anon_rx)
        .await
        .contains("Authentication required"));
    assert!(
        stream_rx.try_recv().is_err(),
        "log stream must keep running"
    );
}

#[tokio::test]
async fn other_connection_cannot_stop_log_stream() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (stream_id, mut stream_rx) = state
        .logs_service
        .insert_test_stream("app", Some(owner))
        .await;
    let (other, mut other_rx) = connect(&state, Some("other@example.com")).await;

    handle_websocket_message(
        &state,
        other,
        &WebSocketMessage::StopLogStream { stream_id },
    )
    .await;

    expect_error(&mut other_rx).await;
    assert!(
        stream_rx.try_recv().is_err(),
        "log stream must keep running"
    );
}

#[tokio::test]
async fn disconnect_ends_owned_sessions_and_streams() {
    let state = app_state().await;
    let (owner, _owner_rx) = connect(&state, Some("owner@example.com")).await;
    let (_session_id, mut session_rx) = state.shell_service.insert_test_session("app", owner).await;
    let (_stream_id, mut stream_rx) = state
        .logs_service
        .insert_test_stream("app", Some(owner))
        .await;
    let (other, _other_rx) = connect(&state, Some("other@example.com")).await;
    let (_other_session, mut other_session_rx) =
        state.shell_service.insert_test_session("app", other).await;

    crate::api::websocket::client::cleanup_client_streams(&state, owner).await;

    assert!(matches!(session_rx.try_recv(), Ok(ShellCommand::Terminate)));
    assert!(matches!(stream_rx.try_recv(), Ok(LogStreamCommand::Stop)));
    assert!(
        other_session_rx.try_recv().is_err(),
        "other clients' sessions must not be touched"
    );
}
