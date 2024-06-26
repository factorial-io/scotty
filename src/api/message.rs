use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum WebSocketMessage {
    Ping,
    Pong,
    Error(String),
}
