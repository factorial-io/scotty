//! Unit tests for CustomAction and ActionStatus

#[cfg(test)]
mod tests {
    use super::super::custom_action::*;
    use crate::authorization::Permission;
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_action() -> CustomAction {
        let mut commands = HashMap::new();
        commands.insert("web".to_string(), vec!["php artisan migrate".to_string()]);

        CustomAction {
            name: "deploy".to_string(),
            description: "Deploy the application".to_string(),
            commands,
            permission: Permission::ActionWrite,
            created_by: "developer@example.com".to_string(),
            created_at: Utc::now(),
            status: ActionStatus::Pending,
            reviewed_by: None,
            reviewed_at: None,
            review_comment: None,
            expires_at: None,
        }
    }

    #[test]
    fn test_action_status_is_executable() {
        assert!(!ActionStatus::Pending.is_executable());
        assert!(ActionStatus::Approved.is_executable());
        assert!(!ActionStatus::Rejected.is_executable());
        assert!(!ActionStatus::Revoked.is_executable());
        assert!(!ActionStatus::Expired.is_executable());
    }

    #[test]
    fn test_action_status_display() {
        assert_eq!(ActionStatus::Pending.to_string(), "pending");
        assert_eq!(ActionStatus::Approved.to_string(), "approved");
        assert_eq!(ActionStatus::Rejected.to_string(), "rejected");
        assert_eq!(ActionStatus::Revoked.to_string(), "revoked");
        assert_eq!(ActionStatus::Expired.to_string(), "expired");
    }

    #[test]
    fn test_action_status_serialization() {
        let json = serde_json::to_string(&ActionStatus::Pending).unwrap();
        assert_eq!(json, "\"pending\"");

        let json = serde_json::to_string(&ActionStatus::Approved).unwrap();
        assert_eq!(json, "\"approved\"");
    }

    #[test]
    fn test_action_status_deserialization() {
        let status: ActionStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(status, ActionStatus::Pending);

        let status: ActionStatus = serde_json::from_str("\"approved\"").unwrap();
        assert_eq!(status, ActionStatus::Approved);
    }

    #[test]
    fn test_custom_action_creation() {
        let action = create_test_action();

        assert_eq!(action.name, "deploy");
        assert_eq!(action.status, ActionStatus::Pending);
        assert_eq!(action.permission, Permission::ActionWrite);
        assert!(action.reviewed_by.is_none());
        assert!(action.reviewed_at.is_none());
    }

    #[test]
    fn test_custom_action_serialization() {
        let action = create_test_action();
        let json = serde_json::to_string(&action).unwrap();

        // Verify key fields are present
        assert!(json.contains("\"name\":\"deploy\""));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"permission\":\"action_write\""));
    }

    #[test]
    fn test_custom_action_deserialization() {
        let json = r#"{
            "name": "test-action",
            "description": "A test action",
            "commands": {"web": ["echo hello"]},
            "permission": "action_read",
            "created_by": "user@example.com",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "approved"
        }"#;

        let action: CustomAction = serde_json::from_str(json).unwrap();

        assert_eq!(action.name, "test-action");
        assert_eq!(action.status, ActionStatus::Approved);
        assert_eq!(action.permission, Permission::ActionRead);
    }

    #[test]
    fn test_custom_action_with_review_info() {
        let mut action = create_test_action();
        action.status = ActionStatus::Approved;
        action.reviewed_by = Some("admin@example.com".to_string());
        action.reviewed_at = Some(Utc::now());
        action.review_comment = Some("Looks good!".to_string());

        let json = serde_json::to_string(&action).unwrap();

        assert!(json.contains("\"status\":\"approved\""));
        assert!(json.contains("\"reviewed_by\":\"admin@example.com\""));
        assert!(json.contains("\"review_comment\":\"Looks good!\""));
    }

    #[test]
    fn test_create_custom_action_request_serialization() {
        let mut commands = HashMap::new();
        commands.insert("web".to_string(), vec!["echo test".to_string()]);

        let request = CreateCustomActionRequest {
            name: "my-action".to_string(),
            description: "Test action".to_string(),
            commands,
            permission: "action_write".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"my-action\""));
        assert!(json.contains("\"permission\":\"action_write\""));
    }

    #[test]
    fn test_create_custom_action_request_deserialization() {
        let json = r#"{
            "name": "deploy",
            "description": "Deploy app",
            "commands": {"web": ["deploy.sh"]},
            "permission": "action_read"
        }"#;

        let request: CreateCustomActionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.name, "deploy");
        assert_eq!(request.permission, "action_read");
    }

    #[test]
    fn test_create_custom_action_request_default_permission() {
        let json = r#"{
            "name": "test",
            "description": "test",
            "commands": {}
        }"#;

        let request: CreateCustomActionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.permission, "action_write");
    }

    #[test]
    fn test_custom_action_list_serialization() {
        let action = create_test_action();
        let list = CustomActionList {
            actions: vec![action],
        };

        let json = serde_json::to_string(&list).unwrap();
        assert!(json.contains("\"actions\":["));
        assert!(json.contains("\"name\":\"deploy\""));
    }

    #[test]
    fn test_review_action_request() {
        let request = ReviewActionRequest {
            comment: Some("Approved after review".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"comment\":\"Approved after review\""));

        // Test with no comment - field is skipped when None due to skip_serializing_if
        let request = ReviewActionRequest { comment: None };
        let json = serde_json::to_string(&request).unwrap();
        assert!(!json.contains("comment")); // Field is omitted, not serialized as null
    }

    #[test]
    fn test_permission_default_is_action_write() {
        // When deserializing without permission field, should default to ActionWrite
        let json = r#"{
            "name": "test",
            "description": "test",
            "commands": {},
            "created_by": "user@example.com",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "pending"
        }"#;

        let action: CustomAction = serde_json::from_str(json).unwrap();
        assert_eq!(action.permission, Permission::ActionWrite);
    }

    // Tests for can_execute() - critical for security
    #[test]
    fn test_can_execute_pending_action_returns_false() {
        let action = create_test_action();
        assert_eq!(action.status, ActionStatus::Pending);
        assert!(
            !action.can_execute(),
            "Pending actions should not be executable"
        );
    }

    #[test]
    fn test_can_execute_approved_action_returns_true() {
        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), None);
        assert_eq!(action.status, ActionStatus::Approved);
        assert!(
            action.can_execute(),
            "Approved actions should be executable"
        );
    }

    #[test]
    fn test_can_execute_rejected_action_returns_false() {
        let mut action = create_test_action();
        action.reject(
            "admin@example.com".to_string(),
            Some("Not allowed".to_string()),
        );
        assert_eq!(action.status, ActionStatus::Rejected);
        assert!(
            !action.can_execute(),
            "Rejected actions should not be executable"
        );
    }

    #[test]
    fn test_can_execute_revoked_action_returns_false() {
        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), None);
        assert!(action.can_execute(), "Should be executable after approval");

        action.revoke(
            "admin@example.com".to_string(),
            Some("No longer needed".to_string()),
        );
        assert_eq!(action.status, ActionStatus::Revoked);
        assert!(
            !action.can_execute(),
            "Revoked actions should not be executable"
        );
    }

    #[test]
    fn test_can_execute_expired_status_returns_false() {
        let mut action = create_test_action();
        action.expire();
        assert_eq!(action.status, ActionStatus::Expired);
        assert!(
            !action.can_execute(),
            "Expired actions should not be executable"
        );
    }

    #[test]
    fn test_can_execute_approved_but_past_expiration_returns_false() {
        use chrono::Duration;

        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), None);

        // Set expiration to 1 hour ago
        action.expires_at = Some(Utc::now() - Duration::hours(1));

        assert_eq!(action.status, ActionStatus::Approved);
        assert!(
            !action.can_execute(),
            "Actions past their expiration should not be executable"
        );
    }

    #[test]
    fn test_can_execute_approved_with_future_expiration_returns_true() {
        use chrono::Duration;

        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), None);

        // Set expiration to 1 hour from now
        action.expires_at = Some(Utc::now() + Duration::hours(1));

        assert_eq!(action.status, ActionStatus::Approved);
        assert!(
            action.can_execute(),
            "Approved actions with future expiration should be executable"
        );
    }

    #[test]
    fn test_approve_sets_reviewer_info() {
        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), Some("LGTM".to_string()));

        assert_eq!(action.status, ActionStatus::Approved);
        assert_eq!(action.reviewed_by, Some("admin@example.com".to_string()));
        assert!(action.reviewed_at.is_some());
        assert_eq!(action.review_comment, Some("LGTM".to_string()));
    }

    #[test]
    fn test_reject_sets_reviewer_info() {
        let mut action = create_test_action();
        action.reject(
            "admin@example.com".to_string(),
            Some("Security concern".to_string()),
        );

        assert_eq!(action.status, ActionStatus::Rejected);
        assert_eq!(action.reviewed_by, Some("admin@example.com".to_string()));
        assert!(action.reviewed_at.is_some());
        assert_eq!(action.review_comment, Some("Security concern".to_string()));
    }

    #[test]
    fn test_revoke_sets_reviewer_info() {
        let mut action = create_test_action();
        action.approve("admin@example.com".to_string(), None);
        action.revoke(
            "security@example.com".to_string(),
            Some("Emergency revocation".to_string()),
        );

        assert_eq!(action.status, ActionStatus::Revoked);
        assert_eq!(action.reviewed_by, Some("security@example.com".to_string()));
        assert!(action.reviewed_at.is_some());
        assert_eq!(
            action.review_comment,
            Some("Emergency revocation".to_string())
        );
    }
}
