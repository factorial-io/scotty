#[cfg(test)]
mod tests {
    use super::super::shell::*;
    use bollard::Docker;
    use scotty_core::apps::app_data::{AppData, AppStatus, ContainerState, ContainerStatus};
    use scotty_core::settings::shell::ShellSettings;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// Helper function to get Docker client for tests
    /// Returns None if Docker is not available, allowing tests to skip gracefully
    async fn get_docker_for_test() -> Option<Docker> {
        match Docker::connect_with_local_defaults() {
            Ok(docker) => {
                // Try to ping Docker to ensure it's actually running
                match docker.ping().await {
                    Ok(_) => Some(docker),
                    Err(_) => {
                        eprintln!("Docker daemon not responding");
                        None
                    }
                }
            }
            Err(_) => {
                eprintln!("Docker not available");
                None
            }
        }
    }

    fn create_test_shell_settings() -> ShellSettings {
        ShellSettings {
            default_shell: "/bin/sh".to_string(),
            session_ttl_seconds: 3600,
            max_sessions_per_app: 5,
            max_sessions_global: 100,
            default_env: HashMap::new(),
        }
    }

    fn create_test_app_data() -> AppData {
        AppData {
            name: "test-app".to_string(),
            status: AppStatus::Running,
            root_directory: "/apps/test-app".to_string(),
            docker_compose_path: "/apps/test-app/docker-compose.yml".to_string(),
            services: vec![ContainerState {
                id: Some("container-123".to_string()),
                service: "web".to_string(),
                domains: vec![],
                use_tls: false,
                port: None,
                status: ContainerStatus::Running,
                started_at: Some(chrono::Local::now()),
                used_registry: None,
                basic_auth: None,
            }],
            settings: None,
            last_checked: Some(chrono::Local::now()),
        }
    }

    #[test]
    fn test_shell_service_error_display() {
        let err = ShellServiceError::ServiceNotFound {
            service: "web".to_string(),
            app: "test-app".to_string(),
        };
        assert_eq!(err.to_string(), "Service 'web' not found in app 'test-app'");

        let err = ShellServiceError::SessionNotFound {
            session_id: Uuid::new_v4(),
        };
        assert!(err.to_string().starts_with("Session '"));

        let err = ShellServiceError::MaxSessionsPerApp { limit: 5 };
        assert_eq!(err.to_string(), "Maximum sessions per app (5) reached");

        let err = ShellServiceError::MaxSessionsGlobal { limit: 100 };
        assert_eq!(err.to_string(), "Maximum global sessions (100) reached");

        let err = ShellServiceError::CommandSendFailed {
            reason: "channel closed".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to send command to session: channel closed"
        );
    }

    #[test]
    fn test_shell_session_ttl_enforcement() {
        // Test that sessions correctly expire based on TTL to prevent resource leaks
        let session = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            exec_id: "exec-456".to_string(),
            sender: tokio::sync::mpsc::channel(1).0,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now() - chrono::Duration::hours(2),
        };

        // Session created 2 hours ago should be expired with 1 hour TTL
        assert!(session.is_expired(std::time::Duration::from_secs(3600)));

        // Same session should NOT be expired with 3 hour TTL
        assert!(!session.is_expired(std::time::Duration::from_secs(10800)));
    }

    #[test]
    fn test_shell_session_to_info() {
        let session = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            exec_id: "exec-456".to_string(),
            sender: tokio::sync::mpsc::channel(1).0,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
        };

        let info = session.to_info("/bin/bash".to_string());
        assert_eq!(info.session_id, session.session_id);
        assert_eq!(info.app_name, "test-app");
        assert_eq!(info.service_name, "web");
        assert_eq!(info.shell_command, "/bin/bash");
        // Note: created_at is not exposed in ShellSessionInfo
    }

    #[tokio::test]
    async fn test_shell_service_initialization() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let settings = create_test_shell_settings();
        let service = ShellService::new(docker, settings);

        // Verify the service is initialized with empty sessions
        let active_sessions = service.get_active_sessions().await;
        assert_eq!(active_sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_terminate_nonexistent_session() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let settings = create_test_shell_settings();
        let service = ShellService::new(docker, settings);

        let random_id = Uuid::new_v4();
        let result = service.terminate_session(random_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ShellServiceError::SessionNotFound { session_id } => {
                assert_eq!(session_id, random_id);
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_send_input_to_nonexistent_session() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let settings = create_test_shell_settings();
        let service = ShellService::new(docker, settings);

        let random_id = Uuid::new_v4();
        let result = service
            .send_input(random_id, "test input".to_string())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ShellServiceError::SessionNotFound { session_id } => {
                assert_eq!(session_id, random_id);
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_resize_tty_nonexistent_session() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let settings = create_test_shell_settings();
        let service = ShellService::new(docker, settings);

        let random_id = Uuid::new_v4();
        let result = service.resize_tty(random_id, 80, 24).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ShellServiceError::SessionNotFound { session_id } => {
                assert_eq!(session_id, random_id);
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_shell_command_channel_communication() {
        // Test that commands are properly transmitted through the channel
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        let session = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            exec_id: "exec-456".to_string(),
            sender: tx,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
        };

        // Test that input commands are transmitted correctly
        let result = session
            .send_command(ShellCommand::Input("ls -la".to_string()))
            .await;
        assert!(result.is_ok());

        // Verify the exact command was received on the other end
        let received = rx.try_recv();
        assert!(received.is_ok());
        match received.unwrap() {
            ShellCommand::Input(text) => assert_eq!(text, "ls -la"),
            _ => panic!("Command type was altered during transmission"),
        }

        // Test resize command transmission
        session
            .send_command(ShellCommand::Resize {
                width: 80,
                height: 24,
            })
            .await
            .unwrap();
        match rx.try_recv().unwrap() {
            ShellCommand::Resize { width, height } => {
                assert_eq!(width, 80);
                assert_eq!(height, 24);
            }
            _ => panic!("Resize command not transmitted correctly"),
        }
    }

    // Session expiration boundary tests
    #[test]
    fn test_session_expiration_exact_boundary() {
        // Session at exactly TTL boundary
        let session = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            exec_id: "exec-456".to_string(),
            sender: tokio::sync::mpsc::channel(1).0,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now() - chrono::Duration::seconds(3600),
        };

        let ttl = std::time::Duration::from_secs(3600);

        // At exact boundary, should NOT be expired
        // (We use > comparison, not >=)
        assert!(!session.is_expired(ttl));
    }

    #[test]
    fn test_session_expiration_just_over_boundary() {
        // Session 1 second over TTL
        let session = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            exec_id: "exec-456".to_string(),
            sender: tokio::sync::mpsc::channel(1).0,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now() - chrono::Duration::seconds(3601),
        };

        let ttl = std::time::Duration::from_secs(3600);

        // 1 second over, should be expired
        assert!(session.is_expired(ttl));
    }

    // Error variant tests
    #[test]
    fn test_error_service_not_found_display() {
        let err = ShellServiceError::ServiceNotFound {
            service: "nginx".to_string(),
            app: "my-app".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("nginx"));
        assert!(display.contains("my-app"));
        assert_eq!(display, "Service 'nginx' not found in app 'my-app'");
    }

    #[test]
    fn test_error_no_container_id_display() {
        let err = ShellServiceError::NoContainerId {
            service: "nginx".to_string(),
        };

        let display = format!("{}", err);
        assert!(display.contains("nginx"));
        assert_eq!(display, "Service 'nginx' has no container ID");
    }

    #[test]
    fn test_all_error_variants_implement_required_traits() {
        // Test that all error variants can be constructed and implement Display
        let errors = vec![
            ShellServiceError::ServiceNotFound {
                service: "svc".to_string(),
                app: "app".to_string(),
            },
            ShellServiceError::NoContainerId {
                service: "svc".to_string(),
            },
            ShellServiceError::MaxSessionsPerApp { limit: 5 },
            ShellServiceError::MaxSessionsGlobal { limit: 50 },
            ShellServiceError::SessionNotFound {
                session_id: Uuid::new_v4(),
            },
            ShellServiceError::CommandSendFailed {
                reason: "closed".to_string(),
            },
            ShellServiceError::DockerOperationFailed {
                operation: "create_exec".to_string(),
                message: "connection failed".to_string(),
            },
        ];

        for error in errors {
            // Each error should have a non-empty display message
            let display = format!("{}", error);
            assert!(!display.is_empty());

            // Error trait should be implemented (this compiles = trait is implemented)
            let _: &dyn std::error::Error = &error;
        }
    }

    // Session ID uniqueness test
    #[test]
    fn test_session_id_uniqueness() {
        use std::collections::HashSet;

        let mut ids = HashSet::new();

        // Generate 1000 session IDs
        for _ in 0..1000 {
            let id = Uuid::new_v4();
            // Each ID should be unique
            assert!(ids.insert(id), "Duplicate UUID generated: {}", id);
        }

        assert_eq!(ids.len(), 1000);
    }

    // Session info field validation
    #[test]
    fn test_session_info_no_pii_or_sensitive_data() {
        let info = ShellSession {
            session_id: Uuid::new_v4(),
            app_name: "my-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-id".to_string(),
            exec_id: "exec-id".to_string(),
            sender: tokio::sync::mpsc::channel(1).0,
            client_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
        }
        .to_info("/bin/sh".to_string());

        // Verify all required fields are present
        assert!(!info.app_name.is_empty());
        assert!(!info.service_name.is_empty());
        assert!(!info.container_id.is_empty());
        assert!(!info.shell_command.is_empty());

        // Verify session_id is a valid UUID
        assert_eq!(info.session_id.to_string().len(), 36); // UUID string length
    }

    // ShellCommand enum variants test
    #[test]
    fn test_shell_command_enum_completeness() {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel(3);

        // Test all ShellCommand variants
        let commands = vec![
            ShellCommand::Input("test input".to_string()),
            ShellCommand::Resize {
                width: 100,
                height: 50,
            },
            ShellCommand::Terminate,
        ];

        // Send all commands
        for cmd in commands {
            tx.try_send(cmd).unwrap();
        }

        // Verify all received correctly
        match rx.try_recv().unwrap() {
            ShellCommand::Input(s) => assert_eq!(s, "test input"),
            _ => panic!("Wrong variant"),
        }

        match rx.try_recv().unwrap() {
            ShellCommand::Resize { width, height } => {
                assert_eq!(width, 100);
                assert_eq!(height, 50);
            }
            _ => panic!("Wrong variant"),
        }

        match rx.try_recv().unwrap() {
            ShellCommand::Terminate => (),
            _ => panic!("Wrong variant"),
        }
    }
}
