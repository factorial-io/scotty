use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::authorization::Permission;

/// Status of a custom action in the approval workflow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action created, awaiting approval
    Pending,
    /// Action approved and can be executed
    Approved,
    /// Action rejected by an approver
    Rejected,
    /// Previously approved action that has been revoked
    Revoked,
    /// Action expired due to TTL
    Expired,
}

impl ActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionStatus::Pending => "pending",
            ActionStatus::Approved => "approved",
            ActionStatus::Rejected => "rejected",
            ActionStatus::Revoked => "revoked",
            ActionStatus::Expired => "expired",
        }
    }

    /// Check if the action can be executed
    pub fn is_executable(&self) -> bool {
        matches!(self, ActionStatus::Approved)
    }
}

impl std::fmt::Display for ActionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A custom action created for a specific app
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CustomAction {
    /// Unique name for this action within the app
    pub name: String,

    /// Human-readable description of what this action does
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Commands to execute per service (service_name -> list of commands)
    pub commands: HashMap<String, Vec<String>>,

    /// Required permission to execute this action (ActionRead or ActionWrite)
    #[serde(default = "default_action_permission")]
    pub permission: Permission,

    /// User who created this action
    pub created_by: String,

    /// When this action was created
    pub created_at: DateTime<Utc>,

    /// Current status in the approval workflow
    pub status: ActionStatus,

    /// User who reviewed this action (approved/rejected/revoked)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_by: Option<String>,

    /// When this action was reviewed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,

    /// Comment from the reviewer
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_comment: Option<String>,

    /// When this action expires (if TTL is configured)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

impl CustomAction {
    /// Create a new custom action in pending status
    pub fn new(
        name: String,
        description: String,
        commands: HashMap<String, Vec<String>>,
        permission: Permission,
        created_by: String,
    ) -> Self {
        Self {
            name,
            description,
            commands,
            permission,
            created_by,
            created_at: Utc::now(),
            status: ActionStatus::Pending,
            reviewed_by: None,
            reviewed_at: None,
            review_comment: None,
            expires_at: None,
        }
    }

    /// Create a new custom action with expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Approve this action
    pub fn approve(&mut self, reviewer: String, comment: Option<String>) {
        self.status = ActionStatus::Approved;
        self.reviewed_by = Some(reviewer);
        self.reviewed_at = Some(Utc::now());
        self.review_comment = comment;
    }

    /// Reject this action
    pub fn reject(&mut self, reviewer: String, comment: Option<String>) {
        self.status = ActionStatus::Rejected;
        self.reviewed_by = Some(reviewer);
        self.reviewed_at = Some(Utc::now());
        self.review_comment = comment;
    }

    /// Revoke a previously approved action
    pub fn revoke(&mut self, reviewer: String, comment: Option<String>) {
        self.status = ActionStatus::Revoked;
        self.reviewed_by = Some(reviewer);
        self.reviewed_at = Some(Utc::now());
        self.review_comment = comment;
    }

    /// Mark this action as expired
    pub fn expire(&mut self) {
        self.status = ActionStatus::Expired;
    }

    /// Check if this action can be executed
    pub fn can_execute(&self) -> bool {
        // Check status
        if !self.status.is_executable() {
            return false;
        }

        // Check expiration
        if let Some(expires_at) = self.expires_at {
            if Utc::now() > expires_at {
                return false;
            }
        }

        true
    }

    /// Get commands for a specific service
    pub fn get_commands_for_service(&self, service: &str) -> Option<&Vec<String>> {
        self.commands.get(service)
    }
}

/// Request to create a new custom action
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCustomActionRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub commands: HashMap<String, Vec<String>>,
    /// Permission as string for API flexibility (e.g., "action_read" or "action_write")
    #[serde(default = "default_permission_string")]
    pub permission: String,
}

fn default_permission_string() -> String {
    "action_write".to_string()
}

fn default_action_permission() -> Permission {
    Permission::ActionWrite
}

/// Response for listing custom actions
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CustomActionList {
    pub actions: Vec<CustomAction>,
}

/// Request to approve/reject/revoke an action
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewActionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}
