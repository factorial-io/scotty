pub mod admin;
pub mod api;
pub mod apps;
pub mod auth;
pub mod authorization;
pub mod http;
pub mod logo;
pub mod notification_types;
pub mod output;
pub mod settings;
pub mod tasks;
pub mod utils;
pub mod version;
pub mod websocket;

// Note: Types previously re-exported here are now imported directly from scotty-types
// This reduces coupling and makes dependencies more explicit
