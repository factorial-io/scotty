use std::sync::Arc;
use tokio::sync::RwLock;

use crate::output::TaskOutput;

// Import the types from scotty-types
pub use scotty_types::{State, TaskDetails};

#[derive(Debug, Clone)]
pub struct TaskState {
    pub handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    pub details: Arc<RwLock<TaskDetails>>,
    pub output: Arc<RwLock<TaskOutput>>,
}
