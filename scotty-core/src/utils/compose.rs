//! Utilities for working with Docker Compose configuration files.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Docker Compose standard configuration files ("compose files") are the file names that Docker Compose
/// automatically recognizes and reads without requiring the `-f` flag.
///
/// **Priority Order:**
/// This list defines the priority order when multiple compose files exist in the same directory.
/// Files are checked in order, and the first existing file is selected. This matches Docker Compose v2's
/// behavior, which prioritizes the newer `compose.yml` naming convention over the legacy
/// `docker-compose.yml` format.
///
/// If you have both v1 and v2 files in the same directory during migration,
/// the v2 files (`compose.yml`/`compose.yaml`) will be preferred. This ensures compatibility with
/// Docker Compose v2's default behavior.
const STD_CONFIG_FILE_NAMES: &[&str] = &[
    "compose.yml",
    "compose.yaml",
    "docker-compose.yml",
    "docker-compose.yaml",
];

/// Finds a standard configuration file in the given directory.
///
/// Returns the path to the first standard configuration file found according to the priority order
/// defined in `STD_CONFIG_FILE_NAMES`, or `None` if none exists.
///
/// If multiple compose files exist in the same directory, only the highest priority file is returned.
pub fn find_config_file_in_dir(dir: &Path) -> Option<PathBuf> {
    for candidate in STD_CONFIG_FILE_NAMES {
        let path = dir.join(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Checks if a file path or filename is a valid standard configuration file.
///
/// This function extracts the filename from the path (if it's a path) and checks if it matches
/// one of the standard compose file names. Works with both full paths and just filenames.
pub fn is_valid_config_file(file_path_or_name: &str) -> bool {
    let file_name = Path::new(file_path_or_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    STD_CONFIG_FILE_NAMES.contains(&file_name)
}

/// Derives the override config file path from the compose file name.
///
/// Docker Compose only auto-merges override files that match the base file's naming convention:
/// - `compose.yaml` merges with `compose.override.yaml`
/// - `docker-compose.yml` merges with `docker-compose.override.yml`
///
/// This function always derives the override file name from the compose file to ensure
/// Docker Compose will automatically merge them.
///
/// For example:
/// - `compose.yml` → `compose.override.yml`
/// - `docker-compose.yaml` → `docker-compose.override.yaml`
pub fn get_override_file(compose_path: &Path) -> Option<PathBuf> {
    let parent_dir = compose_path.parent()?;
    let file_stem = compose_path.file_stem()?.to_str()?;
    let extension = compose_path.extension()?.to_str()?;

    // Build the override filename by inserting .override before the extension
    let override_file_name = format!("{}.override.{}", file_stem, extension);

    Some(parent_dir.join(override_file_name))
}

/// Finds all standard configuration files in a directory tree, returning only the highest priority file per directory.
///
/// This function walks through the directory structure starting from `root_folder` up to `max_depth`
/// levels deep, and collects paths to standard configuration files. If multiple compose files exist
/// in the same directory, only the highest priority file (according to `STD_CONFIG_FILE_NAMES` order)
/// is returned.
///
/// # Arguments
///
/// * `root_folder` - The root directory to start traversal from
/// * `max_depth` - Maximum depth to traverse (0 = only root, 1 = root + one level, etc.)
///
/// # Returns
///
/// A vector of paths to the highest priority configuration file found in each directory.
pub fn find_all_config_files(root_folder: &Path, max_depth: u32) -> Vec<PathBuf> {
    use std::collections::HashMap;

    // Map directory -> (priority_index, file_path)
    let mut dir_files: HashMap<PathBuf, (usize, PathBuf)> = HashMap::new();

    for entry in WalkDir::new(root_folder)
        .max_depth(max_depth as usize)
        .into_iter()
        .flatten()
    {
        if entry.file_type().is_file() {
            if let Some(file_name) = entry.file_name().to_str() {
                if let Some(priority) = STD_CONFIG_FILE_NAMES
                    .iter()
                    .position(|&name| name == file_name)
                {
                    let dir = entry.path().parent().unwrap().to_path_buf();
                    let file_path = entry.path().to_path_buf();

                    // Keep only the file with the highest priority (lowest index) per directory
                    dir_files
                        .entry(dir.clone())
                        .and_modify(|(existing_priority, existing_path)| {
                            if priority < *existing_priority {
                                *existing_priority = priority;
                                *existing_path = file_path.clone();
                            }
                        })
                        .or_insert((priority, file_path));
                }
            }
        }
    }

    dir_files.into_values().map(|(_, path)| path).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_config_file() {
        assert!(is_valid_config_file("docker-compose.yml"));
        assert!(is_valid_config_file("docker-compose.yaml"));
        assert!(is_valid_config_file("compose.yml"));
        assert!(is_valid_config_file("compose.yaml"));
        assert!(is_valid_config_file("./docker-compose.yaml"));
        assert!(is_valid_config_file("./docker-compose.yml"));
        assert!(is_valid_config_file("./compose.yaml"));
        assert!(is_valid_config_file("./compose.yml"));
    }

    #[test]
    fn test_get_override_file() {
        // Always derives override filename from compose filename
        assert_eq!(
            get_override_file(Path::new("/app/docker-compose.yml")).unwrap(),
            Path::new("/app/docker-compose.override.yml")
        );
        assert_eq!(
            get_override_file(Path::new("/app/docker-compose.yaml")).unwrap(),
            Path::new("/app/docker-compose.override.yaml")
        );
        assert_eq!(
            get_override_file(Path::new("/app/compose.yml")).unwrap(),
            Path::new("/app/compose.override.yml")
        );
        assert_eq!(
            get_override_file(Path::new("/app/compose.yaml")).unwrap(),
            Path::new("/app/compose.override.yaml")
        );
    }

    #[test]
    fn test_find_all_config_files_finds_all_variants() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create directory structure with various compose files
        let app1_dir = root.join("app1");
        std::fs::create_dir_all(&app1_dir).unwrap();
        std::fs::write(app1_dir.join("docker-compose.yml"), "services: {}").unwrap();

        let app2_dir = root.join("app2");
        std::fs::create_dir_all(&app2_dir).unwrap();
        std::fs::write(app2_dir.join("compose.yml"), "services: {}").unwrap();

        let app3_dir = root.join("app3");
        std::fs::create_dir_all(&app3_dir).unwrap();
        std::fs::write(app3_dir.join("docker-compose.yaml"), "services: {}").unwrap();

        let app4_dir = root.join("app4");
        std::fs::create_dir_all(&app4_dir).unwrap();
        std::fs::write(app4_dir.join("compose.yaml"), "services: {}").unwrap();

        // Create a directory with a non-compose file (should be ignored)
        let app5_dir = root.join("app5");
        std::fs::create_dir_all(&app5_dir).unwrap();
        std::fs::write(app5_dir.join("other.yml"), "content").unwrap();

        // Create nested directory structure with compose file (root/level1/level2/app/compose.yml = depth 4)
        let nested_dir = root.join("level1").join("level2").join("app");
        std::fs::create_dir_all(&nested_dir).unwrap();
        std::fs::write(nested_dir.join("compose.yml"), "services: {}").unwrap();

        let result = find_all_config_files(root, 4);

        // Should find 5 directories, each with one compose file (highest priority per directory)
        assert_eq!(result.len(), 5);

        // Verify all compose files are found (one per directory)
        let found_files: Vec<String> = result
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(found_files.contains(&"docker-compose.yml".to_string()));
        assert!(found_files.contains(&"compose.yml".to_string()));
        assert!(found_files.contains(&"docker-compose.yaml".to_string()));
        assert!(found_files.contains(&"compose.yaml".to_string()));
        assert!(!found_files.contains(&"other.yml".to_string()));
    }

    #[test]
    fn test_find_all_config_files_priority_per_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a directory with multiple compose files - should only return highest priority
        let app_dir = root.join("app");
        std::fs::create_dir_all(&app_dir).unwrap();
        std::fs::write(app_dir.join("docker-compose.yml"), "services: {}").unwrap();
        std::fs::write(app_dir.join("compose.yml"), "services: {}").unwrap();
        std::fs::write(app_dir.join("docker-compose.yaml"), "services: {}").unwrap();
        std::fs::write(app_dir.join("compose.yaml"), "services: {}").unwrap();

        // max_depth=2 needed to find files in root/app (root=0, app=1, file=2)
        let result = find_all_config_files(root, 2);

        // Should only return one file (highest priority: compose.yml)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_name().unwrap(), "compose.yml");
    }

    #[test]
    fn test_find_config_file_in_dir_priority_order() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path();

        // Create all compose file variants - the highest priority (compose.yml) should be selected
        std::fs::write(test_dir.join("docker-compose.yml"), "services: {}").unwrap();
        std::fs::write(test_dir.join("docker-compose.yaml"), "services: {}").unwrap();
        std::fs::write(test_dir.join("compose.yaml"), "services: {}").unwrap();
        std::fs::write(test_dir.join("compose.yml"), "services: {}").unwrap();

        let found = find_config_file_in_dir(test_dir).unwrap();
        assert_eq!(
            found.file_name().unwrap(),
            "compose.yml",
            "compose.yml should be preferred (highest priority) when all variants exist"
        );
    }

    #[test]
    fn test_find_config_file_in_dir_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let test_dir = temp_dir.path();

        // Directory exists but has no compose files
        assert_eq!(find_config_file_in_dir(test_dir), None);

        // Directory doesn't exist
        let non_existent = test_dir.join("non_existent");
        assert_eq!(find_config_file_in_dir(&non_existent), None);
    }

    #[test]
    fn test_get_override_file_no_extension() {
        // Files without extensions should return None
        assert_eq!(get_override_file(Path::new("compose")), None);
        assert_eq!(get_override_file(Path::new("/some/path/file")), None);
    }

    #[test]
    fn test_find_all_config_files_max_depth_zero() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create compose file in root
        std::fs::write(root.join("compose.yml"), "services: {}").unwrap();

        // Create nested directory with compose file (should not be found with max_depth=0)
        let nested = root.join("subdir");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("compose.yml"), "services: {}").unwrap();

        let result = find_all_config_files(root, 0);

        // Test actual behavior: WalkDir with max_depth=0 only visits the directory entry itself,
        // not files inside it. So no files are found. If we want files in root, we need max_depth=1.
        // This matches standard Unix tool semantics (find -maxdepth 0 doesn't traverse).
        assert_eq!(
            result.len(),
            0,
            "max_depth=0 only visits root directory entry, not files inside"
        );
    }

    #[test]
    fn test_find_all_config_files_max_depth_one_finds_root_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create compose file in root
        std::fs::write(root.join("compose.yml"), "services: {}").unwrap();

        // Create nested directory with compose file (should not be found with max_depth=1)
        let nested = root.join("subdir");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("compose.yml"), "services: {}").unwrap();

        let result = find_all_config_files(root, 1);

        // max_depth=1 finds files directly in root (root + immediate children)
        assert_eq!(
            result.len(),
            1,
            "max_depth=1 should find files in root directory"
        );
        assert_eq!(
            result[0].file_name().unwrap(),
            "compose.yml",
            "Should find compose.yml in root"
        );
    }
}
