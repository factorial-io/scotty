use std::sync::atomic::{AtomicI64, Ordering};

/// WebSocket metrics instrumentation helpers
///
/// These functions provide a clean API for recording WebSocket metrics
/// without cluttering the business logic with metrics implementation details.
///
/// Track active WebSocket connections in memory
static ACTIVE_CONNECTIONS: AtomicI64 = AtomicI64::new(0);

/// Record a new WebSocket connection opened
pub fn record_connection_opened() {
    let count = ACTIVE_CONNECTIONS.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(m) = super::get_metrics() {
        m.websocket_connections_active.record(count, &[]);
    }
}

/// Record a WebSocket connection closed
pub fn record_connection_closed() {
    let count = ACTIVE_CONNECTIONS.fetch_sub(1, Ordering::Relaxed) - 1;
    if let Some(m) = super::get_metrics() {
        m.websocket_connections_active.record(count, &[]);
    }
}

/// Record a message sent to a WebSocket client
pub fn record_message_sent() {
    if let Some(m) = super::get_metrics() {
        m.websocket_messages_sent.add(1, &[]);
    }
}

/// Record multiple messages sent to WebSocket clients (e.g., broadcasts)
pub fn record_messages_sent(count: u64) {
    if count > 0 {
        if let Some(m) = super::get_metrics() {
            m.websocket_messages_sent.add(count, &[]);
        }
    }
}

/// Record a message received from a WebSocket client
pub fn record_message_received() {
    if let Some(m) = super::get_metrics() {
        m.websocket_messages_received.add(1, &[]);
    }
}

/// Record a WebSocket authentication failure
pub fn record_auth_failure() {
    if let Some(m) = super::get_metrics() {
        m.websocket_auth_failures.add(1, &[]);
    }
}
