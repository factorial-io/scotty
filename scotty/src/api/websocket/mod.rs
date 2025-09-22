pub mod client;
pub mod handlers;
pub mod messaging;

// Re-export main types for easy access
pub use messaging::{ClientInfo, WebSocketClients, WebSocketError, WebSocketMessenger};

// Note: WebSocket message types are now in scotty-core::websocket::message
