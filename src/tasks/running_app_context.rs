use serde::{Deserialize, Serialize};
use utoipa::ToResponse;

use crate::apps::app_data::AppData;

use super::task_details::TaskDetails;

#[derive(Debug, Serialize, Deserialize, Default, utoipa::ToSchema, ToResponse)]
pub struct RunningAppContext {
    pub task: TaskDetails,
    pub app_data: AppData,
}

impl RunningAppContext {
    pub fn new(app_data: AppData, task: TaskDetails) -> Self {
        Self { task, app_data }
    }

    pub fn docker_compose_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.app_data.docker_compose_path)
    }
}
