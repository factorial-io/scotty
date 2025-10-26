//! Shell session metrics helper functions
//!
//! This module provides clean helper functions for recording shell session metrics
//! without cluttering business logic.

use std::sync::atomic::{AtomicI64, Ordering};

/// Track active shell sessions in memory
static ACTIVE_SESSIONS: AtomicI64 = AtomicI64::new(0);

/// Record a shell session started
pub fn record_session_started() {
    let count = ACTIVE_SESSIONS.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(m) = super::get_metrics() {
        m.shell_sessions_active.record(count, &[]);
        m.shell_sessions_total.add(1, &[]);
    }
}

/// Record a shell session ended normally
pub fn record_session_ended(duration_secs: f64) {
    let count = ACTIVE_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
    if let Some(m) = super::get_metrics() {
        m.shell_sessions_active.record(count, &[]);
        m.shell_session_duration.record(duration_secs, &[]);
    }
}

/// Record a shell session timeout
pub fn record_session_timeout(duration_secs: f64) {
    let count = ACTIVE_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
    if let Some(m) = super::get_metrics() {
        m.shell_sessions_active.record(count, &[]);
        m.shell_session_duration.record(duration_secs, &[]);
        m.shell_session_timeouts.add(1, &[]);
    }
}

/// Record a shell session error
pub fn record_session_error(duration_secs: f64) {
    let count = ACTIVE_SESSIONS.fetch_sub(1, Ordering::Relaxed) - 1;
    if let Some(m) = super::get_metrics() {
        m.shell_sessions_active.record(count, &[]);
        m.shell_session_duration.record(duration_secs, &[]);
        m.shell_session_errors.add(1, &[]);
    }
}
