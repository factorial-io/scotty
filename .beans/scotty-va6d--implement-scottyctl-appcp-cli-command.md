---
# scotty-va6d
title: Implement scottyctl app:cp CLI command
status: todo
type: task
priority: high
created_at: 2025-12-21T12:44:44Z
updated_at: 2025-12-21T12:44:47Z
parent: scotty-kqlr
---

# Description  Add client-side file copy command with tar handling.  **Implementation** (scottyctl/src/commands/apps/cp.rs):  1. Parse source/destination:    - Container format: "myapp:web:/path" or "myapp:/path" (default service)    - Local format: "./local/path"  2. Create tar for upload: ```rust async fn create_tar_archive(local_path: &str) -> Result<Vec<u8>> {     let mut archive_data = Vec::new();     let mut builder = tar::Builder::new(Cursor::new(&mut archive_data));          if path.is_file() {         builder.append_path_with_name(&path, path.file_name().unwrap())?;     } else if path.is_dir() {         builder.append_dir_all(".", &path)?;     }     builder.finish()?;     Ok(archive_data) } ```  3. Extract tar for download: ```rust async fn extract_tar_archive(bytes: Bytes, local_path: &str) -> Result<()> {     let mut archive = tar::Archive::new(Cursor::new(bytes));     archive.unpack(local_path)?;     Ok(()) } ```  **CLI structure**: ```rust #[derive(Debug, Parser)] pub struct CopyCommand {     pub source: String,     pub destination: String, } ```  **Testing**: - Test file upload: scottyctl app:cp ./file.txt myapp:web:/tmp/ - Test file download: scottyctl app:cp myapp:web:/tmp/file.txt ./ - Test directory copy both directions - Test invalid path format parsing  **Dependencies**: Add to Cargo.toml: tar = "0.4", urlencoding = "2.1"  **Time estimate**: 3-4 hours
