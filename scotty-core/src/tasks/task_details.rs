use std::sync::Arc;
use tokio::sync::RwLock;

// Import the types from scotty-types (which now has embedded TaskOutput)
pub use scotty_types::{State, TaskDetails, TaskOutput};

/// TaskState with embedded output via TaskDetails
#[derive(Debug, Clone)]
pub struct TaskState {
    pub handle: Option<Arc<RwLock<tokio::task::JoinHandle<()>>>>,
    pub details: Arc<RwLock<TaskDetails>>,
}
