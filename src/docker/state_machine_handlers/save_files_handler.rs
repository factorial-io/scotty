use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{info, instrument};

use crate::{apps::file_list::FileList, state_machine::StateHandler};

use super::context::Context;

#[derive(Debug)]
pub struct SaveFilesHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub files: FileList,
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for SaveFilesHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let root_directory = std::path::PathBuf::from(&context.app_data.root_directory);

        for file in &self.files.files {
            let file_path = root_directory.join(&file.name);
            let file_path = path_clean::clean(&file_path);
            if !file_path.starts_with(&root_directory) {
                return Err(anyhow::anyhow!(
                    "Attempted directory traversal attack detected"
                ));
            }

            // Make sure the parent directory exists
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }

            info!("Saving file {} to {}", &file.name, file_path.display());
            tokio::fs::write(&file_path, &file.content).await?;
        }

        Ok(self.next_state.clone())
    }
}
