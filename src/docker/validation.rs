use crate::{api::error::AppError, apps::app_data::ServicePortMapping};

pub fn validate_docker_compose_content(
    docker_compose_content: &[u8],
    public_services: &Vec<ServicePortMapping>,
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
    for public_service in public_services {
        if !available_services.contains(&public_service.service) {
            return Err(AppError::PublicServiceNotFound(
                public_service.service.clone(),
            ));
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
    let content_str = String::from_utf8_lossy(docker_compose_content);
    let env_var_regex = regex::Regex::new(r"\$\{?\w+\}?").unwrap();
    if let Some(found_match) = env_var_regex.find(&content_str) {
        return Err(AppError::EnvironmentVariablesNotSupported(
            found_match.as_str().to_string(),
        ));
    }

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
        let public_services = vec![ServicePortMapping {
            service: "non_existent_service".to_string(),
            port: 80,
        }];
        let result = validate_docker_compose_content(content, &public_services);
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
        let result = validate_docker_compose_content(content, &vec![]);
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
        let result = validate_docker_compose_content(content, &vec![]);
        assert!(matches!(
            result,
            Err(AppError::EnvironmentVariablesNotSupported(var)) if var == "${SOME_VAR}"
        ));
    }

    #[test]
    fn test_valid_docker_compose() {
        let content = b"
services:
  service1:
    image: test
";
        let result = validate_docker_compose_content(content, &vec![]);
        assert!(result.is_ok());
    }
}
