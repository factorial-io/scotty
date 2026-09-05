use scotty_core::apps::{
    app_data::{AppData, AppSettings},
    file_list::FileList,
};
use tracing::{info, instrument};

use super::context::Context;

#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn create_directory(ctx: &Context) -> anyhow::Result<()> {
    let root_directory = std::path::PathBuf::from(&ctx.app_data.root_directory);
    info!("Creating directory {}", root_directory.display());
    if !root_directory.exists() {
        std::fs::create_dir_all(root_directory)?;
    }
    Ok(())
}

#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn remove_directory(ctx: &Context) -> anyhow::Result<()> {
    let root_directory = std::path::PathBuf::from(&ctx.app_data.root_directory);
    info!("Removing directory {}", root_directory.display());
    if root_directory.exists() {
        std::fs::remove_dir_all(&root_directory)?;
    }
    Ok(())
}

#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn save_settings(ctx: &Context, settings: &AppSettings) -> anyhow::Result<()> {
    let app = AppData {
        settings: Some(settings.clone()),
        ..ctx.app_data.clone()
    };
    app.save_settings().await
}

#[instrument(skip_all, fields(app = %ctx.app_data.name))]
pub async fn save_files(ctx: &Context, files: &FileList) -> anyhow::Result<()> {
    let root_directory = std::path::PathBuf::from(&ctx.app_data.root_directory);

    for file in &files.files {
        let file_path = path_clean::clean(root_directory.join(&file.name));
        if !file_path.starts_with(&root_directory) {
            return Err(anyhow::anyhow!(
                "Attempted directory traversal attack detected"
            ));
        }
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        info!("Saving file {} to {}", &file.name, file_path.display());
        tokio::fs::write(&file_path, &file.content).await?;
    }
    Ok(())
}
