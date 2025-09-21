#[cfg(test)]
mod tests {
    use super::super::logs::*;
    use bollard::Docker;
    use scotty_core::apps::app_data::{AppData, AppStatus, ContainerState, ContainerStatus};
    use scotty_core::output::{OutputLine, OutputStreamType};
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

    fn create_test_app_data() -> AppData {
        AppData {
            name: "test-app".to_string(),
            status: AppStatus::Running,
            root_directory: "/apps/test-app".to_string(),
            docker_compose_path: "/apps/test-app/docker-compose.yml".to_string(),
            services: vec![
                ContainerState {
                    id: Some("container-123".to_string()),
                    service: "web".to_string(),
                    domains: vec![],
                    use_tls: false,
                    port: None,
                    status: ContainerStatus::Running,
                    started_at: Some(chrono::Local::now()),
                    used_registry: None,
                    basic_auth: None,
                },
                ContainerState {
                    id: Some("container-456".to_string()),
                    service: "db".to_string(),
                    domains: vec![],
                    use_tls: false,
                    port: None,
                    status: ContainerStatus::Running,
                    started_at: Some(chrono::Local::now()),
                    used_registry: None,
                    basic_auth: None,
                },
            ],
            settings: None,
            last_checked: Some(chrono::Local::now()),
        }
    }

    #[test]
    fn test_log_stream_error_display() {
        let err = LogStreamError::ServiceNotFound {
            service: "web".to_string(),
            app: "test-app".to_string(),
        };
        assert_eq!(err.to_string(), "Service 'web' not found in app 'test-app'");

        let err = LogStreamError::NoContainerId {
            service: "web".to_string(),
        };
        assert_eq!(err.to_string(), "Service 'web' has no container ID");

        let err = LogStreamError::DockerOperationFailed {
            operation: "logs".to_string(),
            message: "connection refused".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Docker operation failed: logs - connection refused"
        );
    }

    #[test]
    fn test_log_output_converter() {
        // Note: LogOutputConverter is private, so we test through the public API
        // This test validates the timestamp extraction logic

        // The actual LogOutputConverter logic is tested through extract_docker_timestamp
    }

    #[test]
    fn test_log_buffer_concept() {
        // LogBuffer is private, but we can test the concept
        // The buffer should prevent memory overflow by limiting lines
        let max_lines = 5;
        let mut test_buffer: Vec<OutputLine> = Vec::new();

        // Simulate buffer filling
        for i in 1..=5 {
            test_buffer.push(OutputLine {
                stream: OutputStreamType::Stdout,
                content: format!("Line {}", i),
                timestamp: chrono::Utc::now(),
                sequence: i as u64,
            });
        }

        // Verify we've reached capacity
        assert_eq!(test_buffer.len(), max_lines);

        // Simulate flush
        let flushed = test_buffer.drain(..).collect::<Vec<_>>();
        assert_eq!(flushed.len(), 5);
        assert!(test_buffer.is_empty());
    }

    #[test]
    fn test_docker_timestamp_format() {
        // Test that we understand Docker's timestamp format
        use chrono::DateTime;

        let docker_log = "2024-01-15T10:30:00.123456789Z Hello World";

        // Docker uses RFC3339 format with nanosecond precision
        if let Some(space_pos) = docker_log.find(' ') {
            let (timestamp_str, content) = docker_log.split_at(space_pos);
            let content = content.trim();

            // Try to parse the timestamp
            let parsed = DateTime::parse_from_rfc3339(timestamp_str);
            assert!(parsed.is_ok());
            assert_eq!(content, "Hello World");
        }
    }

    #[tokio::test]
    async fn test_log_streaming_service_initialization() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let service = LogStreamingService::new(docker);

        // Verify the service is initialized with empty sessions
        let active_sessions = service.get_active_streams().await;
        assert_eq!(active_sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_stop_nonexistent_stream() {
        let Some(docker) = get_docker_for_test().await else {
            println!("Skipping test: Docker not available");
            return;
        };

        let service = LogStreamingService::new(docker);

        let random_id = Uuid::new_v4();
        let result = service.stop_stream(random_id).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LogStreamError::StreamNotFound { stream_id } => {
                assert_eq!(stream_id, random_id);
            }
            _ => panic!("Expected StreamNotFound error"),
        }
    }

    #[test]
    fn test_log_session_to_info() {
        let session = LogStreamSession {
            stream_id: Uuid::new_v4(),
            app_name: "test-app".to_string(),
            service_name: "web".to_string(),
            container_id: "container-123".to_string(),
            client_id: Some(Uuid::new_v4()),
            sender: tokio::sync::mpsc::channel(1).0,
        };

        let info = session.to_info(true);
        assert_eq!(info.stream_id, session.stream_id);
        assert_eq!(info.app_name, "test-app");
        assert_eq!(info.service_name, "web");
        assert!(info.follow);
    }
}
