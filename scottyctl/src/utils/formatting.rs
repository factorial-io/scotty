use anyhow;
use chrono::TimeDelta;
use dotenvy;
use scotty_core::{apps::app_data::AppStatus, utils::format::format_chrono_duration};

pub fn format_since(duration: &Option<TimeDelta>) -> String {
    match duration {
        Some(d) => format_chrono_duration(d),
        None => "N/A".to_string(),
    }
}

pub fn colored_by_status(name: &str, status: &AppStatus) -> String {
    use owo_colors::OwoColorize;
    match status {
        AppStatus::Starting | AppStatus::Running => name.green().to_string(),
        AppStatus::Stopped => name.blue().to_string(),
        AppStatus::Creating => name.bright_green().to_string(),
        AppStatus::Destroying => name.bright_red().to_string(),
        AppStatus::Unsupported => name.white().to_string(),
    }
}

pub fn parse_env_file(file_path: &str) -> anyhow::Result<Vec<(String, String)>> {
    // Use dotenvy to parse the .env file
    let env_vars = dotenvy::from_path_iter(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse env file: {}", e))?;

    // Convert to Vec<(String, String)>
    let env_vars: Vec<(String, String)> = env_vars
        .filter_map(|result| {
            result
                .map_err(|e| {
                    eprintln!("Warning: {}", e);
                    e
                })
                .ok()
        })
        .collect();

    Ok(env_vars)
}

pub fn collect_files(
    docker_compose_path: &str,
) -> anyhow::Result<scotty_core::apps::file_list::FileList> {
    use scotty_core::apps::file_list::{File, FileList};
    use tracing::info;
    use walkdir::WalkDir;

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
