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
    // Build absolute path to frontend generated types folder
    // CARGO_MANIFEST_DIR points to the ts-generator directory, parent is workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("Failed to get workspace root");
    let export_dir = workspace_root.join("frontend/src/generated");

    // Set the export directory for ts-rs
    std::env::set_var("TS_RS_EXPORT_DIR", export_dir.to_str().unwrap());

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
    println!("📁 Generated files location: {}", export_dir.display());

    // List generated files
    let generated_dir = &export_dir;
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
