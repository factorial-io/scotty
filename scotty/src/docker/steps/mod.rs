//! Steps of app lifecycle operations. Each step is a plain `async fn` taking
//! the shared [`Context`]; operations in `docker/*_app.rs` sequence them with
//! `?` as the only error edge and `helper::run_operation` owns the task.

pub mod compose;
pub mod context;
pub mod files;
pub mod load_balancer;
pub mod network;
pub mod post_actions;
pub mod run_task_and_wait;
pub mod wait_for_containers;
