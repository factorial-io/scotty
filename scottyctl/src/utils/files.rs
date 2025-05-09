use anyhow;
use scotty_core::apps::file_list::{File, FileList};
use tracing::info;
use walkdir::WalkDir;

/// Collects all files in the directory containing the docker-compose file.
/// 
/// This function walks through the directory tree and collects all files,
/// supporting both text and binary files. Files like `.DS_Store` and files
/// in `.git` directories are ignored.
/// 
/// # Arguments
/// 
/// * `docker_compose_path` - Path to the docker-compose.yml file
/// 
/// # Returns
/// 
/// A `FileList` containing all collected files with their contents
pub fn collect_files(
    docker_compose_path: &str,
) -> anyhow::Result<FileList> {
    let folder = std::path::Path::new(docker_compose_path)
        .parent()
        .unwrap()
        .to_str()
        .unwrap();
    let mut files = vec![];
    for entry in WalkDir::new(folder) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let file_name = entry.file_name().to_str().unwrap();
            if file_name == ".DS_Store" || entry.path().to_str().unwrap().contains("/.git/") {
                info!("Ignoring file {:?}", entry);
                continue;
            }
            info!("Reading file {:?}", entry);
            let path = entry.path().to_str().unwrap().to_string();
            // Read file as bytes to support binary files
            let content = std::fs::read(&path)?;
            let relative_path = path.replace(folder, ".");
            files.push(File {
                name: relative_path,
                content,
            });
        }
    }
    Ok(FileList { files })
}