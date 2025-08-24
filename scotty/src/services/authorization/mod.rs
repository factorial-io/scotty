//! Authorization module for Scotty
//! 
//! This module provides a comprehensive RBAC (Role-Based Access Control) system
//! using Casbin for policy enforcement. It manages users, roles, groups, and
//! permissions for application access control.

pub mod casbin;
pub mod config;
pub mod fallback;
pub mod service;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export the main types and service for easy access
pub use service::AuthorizationService;
pub use types::{Assignment, AuthConfig, GroupConfig, Permission, PermissionOrWildcard, RoleConfig};