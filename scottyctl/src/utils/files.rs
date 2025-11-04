use anyhow;
use ignore::gitignore::GitignoreBuilder;
use scotty_core::apps::file_list::{File, FileList};
use std::path::Path;
use tracing::info;
use walkdir::WalkDir;

/// Collects all files in the directory containing the docker-compose file.
///
/// This function walks through the directory tree and collects all files,
/// supporting both text and binary files. Files matching patterns in
/// `.scottyignore` are excluded. Files like `.DS_Store` and files in
/// `.git` directories are always ignored.
///
/// # .scottyignore File
///
/// If a `.scottyignore` file exists in the source folder, it will be used
/// to filter files using gitignore-style patterns:
/// - `*.log` - Ignore all .log files
/// - `target/` - Ignore target directory
/// - `!important.log` - Re-include specific file
/// - `**/*.tmp` - Ignore .tmp files in subdirectories
///
/// # Arguments
///
/// * `docker_compose_path` - Path to the docker-compose.yml file
///
/// # Returns
///
/// A `FileList` containing all collected files with their contents
pub fn collect_files(docker_compose_path: &str) -> anyhow::Result<FileList> {
    let folder = std::path::Path::new(docker_compose_path)
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid docker-compose path"))?;

    // Check for .scottyignore in source folder
    let scottyignore_path = folder.join(".scottyignore");
    let scottyignore = if scottyignore_path.exists() {
        info!("Found .scottyignore at {:?}", scottyignore_path);
        let mut builder = GitignoreBuilder::new(folder);
        if let Some(e) = builder.add(&scottyignore_path) {
            tracing::warn!("Failed to read .scottyignore, ignoring file: {}", e);
            None
        } else {
            match builder.build() {
                Ok(ignore) => Some(ignore),
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse .scottyignore, ignoring file: {}",
                        e
                    );
                    None
                }
            }
        }
    } else {
        info!("No .scottyignore found in source folder");
        None
    };

    let mut files = vec![];
    for entry in WalkDir::new(folder) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let relative_path = entry.path().strip_prefix(folder)?;

            // Check .scottyignore patterns
            if let Some(ref ignore) = scottyignore {
                // Check the file itself
                let matched = ignore.matched(relative_path, false);
                if matched.is_ignore() {
                    info!("Ignoring file (scottyignore): {:?}", relative_path);
                    continue;
                }

                // Also check if any parent directory is ignored
                let mut should_ignore = false;
                for ancestor in relative_path.ancestors() {
                    if ancestor == Path::new("") {
                        break;
                    }
                    let dir_matched = ignore.matched(ancestor, true);
                    if dir_matched.is_ignore() {
                        info!("Ignoring file (scottyignore parent): {:?}", relative_path);
                        should_ignore = true;
                        break;
                    }
                }
                if should_ignore {
                    continue;
                }
            }

            // Hardcoded filters (always applied)
            let file_name = entry.file_name().to_str().unwrap_or("");
            if is_hardcoded_ignore(file_name, entry.path()) {
                info!("Ignoring file (hardcoded): {:?}", entry);
                continue;
            }

            info!("Reading file {:?}", entry);
            let content = std::fs::read(entry.path())?;
            let relative_path_str = format!("./{}", relative_path.display());
            files.push(File {
                name: relative_path_str,
                content,
            });
        }
    }
    Ok(FileList { files })
}

/// Check if file should be ignored by hardcoded rules
fn is_hardcoded_ignore(file_name: &str, path: &Path) -> bool {
    file_name == ".DS_Store" || path.to_str().unwrap_or("").contains("/.git/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_files_no_scottyignore() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "fn main() {}").unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert_eq!(files.files.len(), 2); // docker-compose.yml + app.rs
    }

    #[test]
    fn test_collect_files_with_scottyignore() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("debug.log"), "logs").unwrap();
        fs::write(temp_dir.path().join(".scottyignore"), "*.log").unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert_eq!(files.files.len(), 3); // docker-compose.yml + app.rs + .scottyignore
        assert!(!files.files.iter().any(|f| f.name.contains("debug.log")));
    }

    #[test]
    fn test_collect_files_scottyignore_directory() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();

        let target_dir = temp_dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("binary"), "data").unwrap();

        fs::write(temp_dir.path().join(".scottyignore"), "target").unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert!(!files.files.iter().any(|f| f.name.contains("target")));
    }

    #[test]
    fn test_collect_files_scottyignore_negation() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join("debug.log"), "logs").unwrap();
        fs::write(temp_dir.path().join("important.log"), "important").unwrap();
        fs::write(
            temp_dir.path().join(".scottyignore"),
            "*.log\n!important.log",
        )
        .unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert!(!files.files.iter().any(|f| f.name.contains("debug.log")));
        assert!(files
            .files
            .iter()
            .any(|f| f.name.contains("important.log")));
    }

    #[test]
    fn test_hardcoded_filters() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join(".DS_Store"), "").unwrap();

        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();
        fs::write(git_dir.join("config"), "").unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert!(!files.files.iter().any(|f| f.name.contains(".DS_Store")));
        assert!(!files.files.iter().any(|f| f.name.contains(".git")));
    }

    #[test]
    fn test_malformed_scottyignore() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "fn main() {}").unwrap();
        // Create a malformed .scottyignore (invalid regex pattern)
        fs::write(temp_dir.path().join(".scottyignore"), "[[[invalid").unwrap();

        // Should still collect files despite malformed .scottyignore
        let result = collect_files(compose_path.to_str().unwrap());
        assert!(result.is_ok());
        let files = result.unwrap();
        assert!(files.files.len() >= 2); // At least docker-compose.yml + app.rs
    }

    #[test]
    fn test_scottyignore_with_comments() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();
        fs::write(temp_dir.path().join("app.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("debug.log"), "logs").unwrap();
        fs::write(
            temp_dir.path().join(".scottyignore"),
            "# Ignore log files\n*.log\n# End of file",
        )
        .unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert!(!files.files.iter().any(|f| f.name.contains("debug.log")));
    }

    #[test]
    fn test_scottyignore_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let compose_path = temp_dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "version: '3'").unwrap();

        let nested_dir = temp_dir.path().join("src").join("temp");
        fs::create_dir_all(&nested_dir).unwrap();
        fs::write(nested_dir.join("cache.tmp"), "data").unwrap();
        fs::write(temp_dir.path().join("src").join("main.rs"), "fn main() {}").unwrap();

        fs::write(temp_dir.path().join(".scottyignore"), "**/*.tmp").unwrap();

        let files = collect_files(compose_path.to_str().unwrap()).unwrap();
        assert!(!files.files.iter().any(|f| f.name.contains("cache.tmp")));
        assert!(files.files.iter().any(|f| f.name.contains("main.rs")));
    }
}
