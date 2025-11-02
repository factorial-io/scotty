//! Scotty library
//!
//! This library exposes public API for integration testing.
//! Most functionality is in the binary, but we expose test utilities
//! and router creation for E2E testing.

// Internal modules
pub mod api;
pub mod app_state;
pub mod docker;
pub mod http;
pub mod init_telemetry;
pub mod metrics;
pub mod notification;
pub mod oauth;
pub mod onepassword;
pub mod services;
pub mod settings;
pub mod state_machine;
pub mod static_files;
pub mod stop_flag;
pub mod tasks;
pub mod utils;

// Re-export commonly used types for tests
pub use app_state::AppState;
