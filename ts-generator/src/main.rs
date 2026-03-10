/*!
 * Standalone TypeScript type generator for Scotty WebSocket messages
 *
 * This is an optimized, minimal binary that only compiles the types needed
 * for TypeScript generation, significantly reducing compile time compared
 * to building the full workspace.
 *
 * Usage: cargo run (from ts-generator directory)
 */

use scotty_types::{
    ts_rs::{Config, TS},
    *,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build absolute path to frontend generated types folder
    // CARGO_MANIFEST_DIR points to the ts-generator directory, parent is workspace root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = std::path::Path::new(&manifest_dir)
        .parent()
        .expect("Failed to get workspace root");
    let export_dir = workspace_root.join("frontend/src/generated");

    let cfg = Config::new().with_out_dir(&export_dir);

    println!("🔧 Generating TypeScript bindings for WebSocket messages...");

    // Core output types
    OutputStreamType::export(&cfg)?;
    OutputLine::export(&cfg)?;

    // Task types
    State::export(&cfg)?;
    TaskDetails::export(&cfg)?;
    TaskOutputData::export(&cfg)?;

    // Log streaming types
    LogStreamRequest::export(&cfg)?;
    LogsStreamInfo::export(&cfg)?;
    LogsStreamData::export(&cfg)?;
    LogsStreamEnd::export(&cfg)?;
    LogsStreamError::export(&cfg)?;

    // Shell session types
    ShellDataType::export(&cfg)?;
    ShellSessionRequest::export(&cfg)?;
    ShellSessionInfo::export(&cfg)?;
    ShellSessionData::export(&cfg)?;
    ShellSessionEnd::export(&cfg)?;
    ShellSessionError::export(&cfg)?;

    // WebSocket message types
    WebSocketMessage::export(&cfg)?;

    println!("✅ TypeScript bindings generated successfully!");
    println!("📁 Generated files location: {}", export_dir.display());

    // List generated files
    let generated_dir = &export_dir;
    let mut ts_files = Vec::new();

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
            let file_name = entry.file_name().to_string_lossy().to_string();
            println!("   - {}", file_name);
            // Store filename without extension for index generation
            if let Some(name_without_ext) = file_name.strip_suffix(".ts") {
                ts_files.push(name_without_ext.to_string());
            }
        }
    }

    // Generate index.ts file that re-exports all types
    println!("\n🔧 Generating index.ts...");
    let index_path = export_dir.join("index.ts");
    let mut index_content = String::from("// Auto-generated index file - do not edit manually\n");
    index_content.push_str("// Re-exports all generated TypeScript types\n\n");

    for file_name in &ts_files {
        index_content.push_str(&format!("export * from './{}';\n", file_name));
    }

    std::fs::write(&index_path, index_content)?;
    println!("✅ Created index.ts with {} exports", ts_files.len());

    Ok(())
}
