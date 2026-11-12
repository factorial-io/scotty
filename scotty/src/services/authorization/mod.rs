//! Authorization module for Scotty
//!
//! This module provides a comprehensive RBAC (Role-Based Access Control) system
//! with a single decision point over compiled policy snapshots. It manages users, roles, groups, and
//! permissions for application access control.

pub mod config;
pub mod decision;
pub mod fallback;
pub mod policy;
pub mod service;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export the main types and service for easy access
pub use service::AuthorizationService;
pub use types::Permission;
