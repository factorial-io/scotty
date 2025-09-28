/*!
 * TypeScript type generation binary for scotty WebSocket messages
 *
 * This binary generates TypeScript interfaces from Rust types using ts-rs.
 * The generated types will be written to frontend/src/generated/
 *
 * Usage:
 *   cargo run --features ts-rs --bin generate-types
 */

#[cfg(feature = "ts-rs")]
use ts_rs::TS;

#[cfg(feature = "ts-rs")]
use scotty_core::{
    output::{OutputLine, OutputStreamType},
    tasks::task_details::{State, TaskDetails},
    websocket::message::{
        LogStreamRequest, LogsStreamData, LogsStreamEnd, LogsStreamError, LogsStreamInfo,
        ShellDataType, ShellSessionData, ShellSessionEnd, ShellSessionError, ShellSessionInfo,
        TaskOutputData, WebSocketMessage,
    },
};

#[cfg(not(feature = "ts-rs"))]
fn main() {
    eprintln!("Error: ts-rs feature is not enabled");
    eprintln!("Run with: cargo run --features ts-rs --bin generate-types");
    std::process::exit(1);
}

#[cfg(feature = "ts-rs")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating TypeScript bindings for WebSocket messages...");

    // Core output types
    OutputStreamType::export()?;
    OutputLine::export()?;

    // Task types
    State::export()?;
    TaskDetails::export()?;
    TaskOutputData::export()?;

    // Log streaming types
    LogStreamRequest::export()?;
    LogsStreamInfo::export()?;
    LogsStreamData::export()?;
    LogsStreamEnd::export()?;
    LogsStreamError::export()?;

    // Shell session types
    ShellDataType::export()?;
    ShellSessionInfo::export()?;
    ShellSessionData::export()?;
    ShellSessionEnd::export()?;
    ShellSessionError::export()?;

    // Main WebSocket message enum
    WebSocketMessage::export()?;

    println!("✅ TypeScript bindings generated successfully!");
    println!("📁 Generated files location: frontend/src/generated/");

    // List generated files
    let generated_dir = std::path::Path::new("frontend/src/generated");
    if generated_dir.exists() {
        println!("\n📋 Generated files:");
        let mut entries: Vec<_> = std::fs::read_dir(generated_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "ts")
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by_key(|entry| entry.file_name());

        for entry in entries {
            println!("   - {}", entry.file_name().to_string_lossy());
        }
    }

    Ok(())
}
