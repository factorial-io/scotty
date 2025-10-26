/*!
 * Standalone TypeScript type generator for Scotty WebSocket messages
 *
 * This is an optimized, minimal binary that only compiles the types needed
 * for TypeScript generation, significantly reducing compile time compared
 * to building the full workspace.
 *
 * Usage: cargo run (from ts-generator directory)
 */

use scotty_types::{ts_rs::TS, *};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set the export directory to the frontend generated types folder
    std::env::set_var("TS_RS_EXPORT_DIR", "../frontend/src/generated");

    println!("🔧 Generating TypeScript bindings for WebSocket messages...");

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

    // WebSocket message types
    WebSocketMessage::export()?;

    println!("✅ TypeScript bindings generated successfully!");
    println!("📁 Generated files location: ../frontend/src/generated/");

    // List generated files
    let generated_dir = std::path::Path::new("../frontend/src/generated");
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
