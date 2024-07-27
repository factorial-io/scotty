use serde::{Deserialize, Serialize};

use crate::apps::app_data::AppData;

use super::task_details::TaskDetails;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithAppData {
    pub task: TaskDetails,
    pub app_data: AppData,
}

impl TaskWithAppData {
    pub fn new(app_data: AppData, task: TaskDetails) -> Self {
        Self { task, app_data }
    }
}
