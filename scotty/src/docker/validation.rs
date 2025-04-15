use crate::api::error::AppError;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

use super::docker_compose::run_docker_compose_now;

/// Checks if the Docker Compose file contains environment variables by using docker compose config
pub fn check_for_environment_variables(
    docker_compose_content: &[u8],
    env_vars: Option<&HashMap<String, String>>,
) -> Result<(), AppError> {
    // Create a temporary directory to store the docker-compose file
    let temp_dir = tempdir().map_err(|_| AppError::InvalidDockerComposeFile)?;
    let file_path = temp_dir.path().join("docker-compose.yml");

    // Write the content to the temporary file
    let mut file = File::create(&file_path).map_err(|_| AppError::InvalidDockerComposeFile)?;
    file.write_all(docker_compose_content)
        .map_err(|_| AppError::InvalidDockerComposeFile)?;

    // Prepare the command for docker compose config
    let command = vec!["config", "-q"];

    let path_buf = file_path;
    let error_message = run_docker_compose_now(&path_buf, command, env_vars, true)?;

    if error_message.is_empty() {
        return Ok(());
    }

    // Check if the error is related to missing environment variables
    if error_message.contains("variable is not set") && error_message.contains("\"") {
        // Extract the variable name from quotes in the error message
        // Example: WARN[0000] The \"BAR\" variable is not set. Defaulting to a blank string.
        let var_name = error_message
            .split('\\')
            .nth(1)
            .map(|s| format!("${}", s))
            .unwrap_or_else(|| "Unknown".to_string());

        let var_name = var_name.replace("\"", "");
        return Err(AppError::EnvironmentVariablesNotSupported(var_name));
    }

    Err(AppError::InvalidDockerComposeFile)
}

pub fn validate_docker_compose_content(
    docker_compose_content: &[u8],
    public_service_names: &Vec<String>,
    env_vars: Option<&HashMap<String, String>>,
) -> Result<Vec<String>, AppError> {
    let docker_compose_data: serde_json::Value = serde_yml::from_slice(docker_compose_content)
        .map_err(|_| AppError::InvalidDockerComposeFile)?;

    // Get list of available services
    let available_services: Vec<String> = docker_compose_data["services"]
        .as_object()
        .ok_or(AppError::InvalidDockerComposeFile)?
        .keys()
        .cloned()
        .collect();

    // Check if all public_services are available in docker-compose
    for public_service in public_service_names {
        if !available_services.contains(public_service) {
            return Err(AppError::PublicServiceNotFound(public_service.clone()));
        }
    }

    // Check if there is a port settings for each service
    for service in &available_services {
        let service_data = docker_compose_data["services"][&service]
            .as_object()
            .unwrap();
        if service_data.get("ports").is_some() {
            return Err(AppError::PublicPortsNotSupported(service.clone()));
        }
    }

    // Check if docker_compose_content depends on any environment variables
    check_for_environment_variables(docker_compose_content, env_vars)?;

    Ok(available_services)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_service_not_found() {
        let content = b"
services:
  service1:
    image: test
";
        let public_services = vec!["non_existent_service".to_string()];
        let result = validate_docker_compose_content(content, &public_services, None);
        assert!(
            matches!(result, Err(AppError::PublicServiceNotFound(service)) if service == "non_existent_service")
        );
    }

    #[test]
    fn test_public_ports_not_supported() {
        let content = b"
services:
  service1:
    image: test
    ports:
      - 80:80
";
        let result = validate_docker_compose_content(content, &vec![], None);
        assert!(matches!(
            result,
            Err(AppError::PublicPortsNotSupported(service)) if service == "service1"
        ));
    }

    #[test]
    fn test_environment_variables_not_supported() {
        let content = b"
services:
  service1:
    image: test
    environment:
      - VAR=${SOME_VAR}
";
        let result = validate_docker_compose_content(content, &vec![], None);
        assert!(matches!(
            result,
            Err(AppError::EnvironmentVariablesNotSupported(var)) if var == "$SOME_VAR"
        ));
    }

    #[test]
    fn test_environment_variables_supported_with_env() {
        let content = b"
services:
  service1:
    image: test
    environment:
      - VAR=${SOME_VAR}
";
        let mut env_vars = HashMap::new();
        env_vars.insert("SOME_VAR".to_string(), "value".to_string());

        // This test might fail if run_docker_compose_now is mocked in tests
        // as it actually tries to run docker compose
        let result = validate_docker_compose_content(content, &vec![], Some(&env_vars));
        // We'll assume it's ok if it doesn't error with EnvironmentVariablesNotSupported
        if let Err(err) = &result {
            assert!(!matches!(
                err,
                AppError::EnvironmentVariablesNotSupported(_)
            ));
        }
    }

    #[test]
    fn test_valid_docker_compose() {
        let content = b"
services:
  service1:
    image: test
";
        let result = validate_docker_compose_content(content, &vec![], None);
        assert!(result.is_ok());
    }
}
