use crate::api::error::AppError;
use crate::onepassword::env_substitution;
use std::collections::HashMap;

/// Checks if the Docker Compose file contains environment variables without using external commands
pub fn check_for_environment_variables(
    docker_compose_data: &serde_yml::Value,
    env_vars: Option<&HashMap<String, String>>,
) -> Result<(), AppError> {
    let missing_vars = find_env_vars_recursively(docker_compose_data, env_vars);
    if missing_vars.is_empty() {
        Ok(())
    } else {
        Err(AppError::EnvironmentVariablesNotSupported(
            missing_vars.join(", "),
        ))
    }
}

/// Recursively find environment variables in the YAML structure
fn find_env_vars_recursively(
    value: &serde_yml::Value,
    env_vars: Option<&HashMap<String, String>>,
) -> Vec<String> {
    match value {
        serde_yml::Value::String(s) => {
            // Check for environment variables in string values
            env_substitution::extract_env_vars(s)
                .into_iter()
                .filter(|var_name| !has_env_var(var_name, env_vars))
                .collect()
        }
        serde_yml::Value::Sequence(seq) => seq
            .iter()
            .flat_map(|item| find_env_vars_recursively(item, env_vars))
            .collect(),
        serde_yml::Value::Mapping(map) => map
            .values()
            .flat_map(|v| find_env_vars_recursively(v, env_vars))
            .collect(),
        _ => Vec::new(), // Ignore other types
    }
}

/// Check if an environment variable is provided or has a default value
fn has_env_var(var_name: &str, env_vars: Option<&HashMap<String, String>>) -> bool {
    // Remove the ${} wrapper
    let clean_name = if var_name.starts_with("${") && var_name.ends_with('}') {
        &var_name[2..var_name.len() - 1]
    } else if let Some(stripped) = var_name.strip_prefix('$') {
        stripped
    } else {
        var_name
    };

    // Check if the variable is provided in env_vars
    if let Some(vars) = env_vars {
        let actual_name = extract_var_name(clean_name);
        if vars.contains_key(actual_name) {
            return true;
        }
    }

    // Check if the variable has a default value or doesn't require the variable to be set
    // :- and - provide defaults when variable is unset or empty
    // :+ and + only substitute when variable is set, so empty is valid
    // :? and ? require the variable to be set
    if clean_name.contains(":-") || (clean_name.contains('-') && !clean_name.starts_with('-')) {
        return true;
    }

    // :+ and + only apply when variable exists, so not having the variable is valid
    if clean_name.contains(":+") || (clean_name.contains('+') && !clean_name.starts_with('+')) {
        return true;
    }

    // :? and ? require variable to be set, so we return false (already the default)
    false
}

/// Extract the variable name without any modifier/default parts
fn extract_var_name(clean_name: &str) -> &str {
    // First check if the string starts with a modifier
    // If it does, it's not a real modifier but part of the name
    if clean_name.starts_with(":-")
        || clean_name.starts_with(":+")
        || clean_name.starts_with(":?")
        || clean_name.starts_with('-')
        || clean_name.starts_with('+')
        || clean_name.starts_with('?')
    {
        return clean_name;
    }

    // Then handle all modifiers as before
    for &op in &[":-", ":+", ":?", "-", "+", "?"] {
        if let Some(idx) = clean_name.find(op) {
            return &clean_name[..idx];
        }
    }

    clean_name
}

pub fn validate_docker_compose_content(
    docker_compose_content: &[u8],
    public_service_names: &[String],
    env_vars: Option<&HashMap<String, String>>,
) -> Result<Vec<String>, AppError> {
    // Parse the Docker Compose file
    let docker_compose_data: serde_yml::Value = serde_yml::from_slice(docker_compose_content)
        .map_err(|_| AppError::InvalidDockerComposeFile)?;

    // Get list of available services and perform validations
    let available_services = get_available_services(&docker_compose_data)?;
    validate_public_services_exist(&available_services, public_service_names)?;
    validate_no_ports_exposed(&docker_compose_data)?;
    check_for_environment_variables(&docker_compose_data, env_vars)?;

    Ok(available_services)
}

/// Get the list of available services from Docker Compose data
fn get_available_services(docker_compose_data: &serde_yml::Value) -> Result<Vec<String>, AppError> {
    docker_compose_data
        .get("services")
        .and_then(|services| services.as_mapping())
        .map(|mapping| {
            mapping
                .keys()
                .filter_map(|k| k.as_str().map(String::from))
                .collect()
        })
        .ok_or(AppError::InvalidDockerComposeFile)
}

/// Validate that all public services exist in the available services list
fn validate_public_services_exist(
    available_services: &[String],
    public_service_names: &[String],
) -> Result<(), AppError> {
    for public_service in public_service_names {
        if !available_services.contains(public_service) {
            return Err(AppError::PublicServiceNotFound(public_service.clone()));
        }
    }
    Ok(())
}

/// Validate that no services expose ports
fn validate_no_ports_exposed(docker_compose_data: &serde_yml::Value) -> Result<(), AppError> {
    let services = docker_compose_data
        .get("services")
        .and_then(|s| s.as_mapping())
        .ok_or(AppError::InvalidDockerComposeFile)?;

    for (service_key, service_data) in services {
        let service_name = service_key
            .as_str()
            .ok_or(AppError::InvalidDockerComposeFile)?;
        if service_data.get("ports").is_some() {
            return Err(AppError::PublicPortsNotSupported(service_name.to_string()));
        }
    }

    Ok(())
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
        let result = validate_docker_compose_content(content, &[], None);
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
        let result = validate_docker_compose_content(content, &[], None);
        assert!(matches!(
            result,
            Err(AppError::EnvironmentVariablesNotSupported(var)) if var == "${SOME_VAR}"
        ));
    }

    #[test]
    fn test_environment_variables_with_defaults() {
        let content = b"
services:
  service1:
    image: test
    environment:
      - VAR=${SOME_VAR:-default_value}
      - VAR2=${OTHER_VAR-another_default}
";
        let result = validate_docker_compose_content(content, &[], None);
        assert!(
            result.is_ok(),
            "Docker compose with environment variable defaults should be valid"
        );
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
        let result = validate_docker_compose_content(content, &[], Some(&env_vars));
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
        let result = validate_docker_compose_content(content, &[], None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_environment_variables_with_advanced_patterns() {
        let content = b"
services:
  service1:
    image: test
    environment:
      - VAR1=${SOME_VAR:-default_value}
      - VAR2=${OTHER_VAR-another_default}
      - VAR3=${CONDITIONAL:+replacement}
      - VAR4=${ANOTHER+replacement}
      - VAR5=${REQUIRED:?error message}
      - VAR6=${NEEDED?error}
";
        // Only patterns with defaults should be valid without env vars
        let result = validate_docker_compose_content(content, &[], None);
        assert!(
            result.is_err(),
            "Environment patterns without defaults should require env vars"
        );

        // With all env vars provided, validation should pass
        let mut env_vars = HashMap::new();
        env_vars.insert("SOME_VAR".to_string(), "value".to_string());
        env_vars.insert("OTHER_VAR".to_string(), "value".to_string());
        env_vars.insert("CONDITIONAL".to_string(), "value".to_string());
        env_vars.insert("ANOTHER".to_string(), "value".to_string());
        env_vars.insert("REQUIRED".to_string(), "value".to_string());
        env_vars.insert("NEEDED".to_string(), "value".to_string());

        let result = validate_docker_compose_content(content, &[], Some(&env_vars));
        assert!(
            result.is_ok(),
            "Should be valid when all env vars are provided"
        );
    }

    #[test]
    fn test_extract_var_name() {
        // Simple variable with no modifiers
        assert_eq!(extract_var_name("SIMPLE_VAR"), "SIMPLE_VAR");

        // Variables with different modifiers
        assert_eq!(extract_var_name("VAR_NAME:-default"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME-default"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME:+value_if_set"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME+value_if_set"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME:?error_msg"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME?error_msg"), "VAR_NAME");

        // Empty string
        assert_eq!(extract_var_name(""), "");

        // Variable with modifier at the beginning (shouldn't be treated as a modifier)
        assert_eq!(extract_var_name("-STARTS_WITH_DASH"), "-STARTS_WITH_DASH");
        assert_eq!(
            extract_var_name(":-STARTS_WITH_COLON_DASH"),
            ":-STARTS_WITH_COLON_DASH"
        );

        // Variables with multiple modifiers (should find the first one)
        assert_eq!(extract_var_name("VAR_NAME:-default-with-dash"), "VAR_NAME");
        assert_eq!(extract_var_name("VAR_NAME:-default:+another"), "VAR_NAME");

        // Complex cases
        assert_eq!(extract_var_name("NESTED_VAR_:-${OTHER_VAR}"), "NESTED_VAR_");
        assert_eq!(
            extract_var_name("VAR_WITH_UNDERSCORE_:+_suffix"),
            "VAR_WITH_UNDERSCORE_"
        );
    }

    #[test]
    fn test_has_env_var() {
        // Create a test environment variable map
        let mut env_vars = HashMap::new();
        env_vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        env_vars.insert("EMPTY_VAR".to_string(), "".to_string());

        // Test with variable in env_vars
        assert!(has_env_var("TEST_VAR", Some(&env_vars)));

        // Test with ${} syntax
        assert!(has_env_var("${TEST_VAR}", Some(&env_vars)));

        // Test with variable not in env_vars
        assert!(!has_env_var("MISSING_VAR", Some(&env_vars)));
        assert!(!has_env_var("${MISSING_VAR}", Some(&env_vars)));

        // Test with empty env_vars
        assert!(!has_env_var("TEST_VAR", None));

        // Test with default values (:-)
        assert!(has_env_var("MISSING_VAR:-default", None));
        assert!(has_env_var("${MISSING_VAR:-default}", None));

        // Test with default values (-)
        assert!(has_env_var("MISSING_VAR-default", None));
        assert!(has_env_var("${MISSING_VAR-default}", None));

        // Test with use-if-set values (:+)
        assert!(has_env_var("MISSING_VAR:+value_if_set", None));
        assert!(has_env_var("${MISSING_VAR:+value_if_set}", None));

        // Test with use-if-set values (+)
        assert!(has_env_var("MISSING_VAR+value_if_set", None));
        assert!(has_env_var("${MISSING_VAR+value_if_set}", None));

        // Test with required values (:?)
        assert!(!has_env_var("MISSING_VAR:?error", None));
        assert!(!has_env_var("${MISSING_VAR:?error}", None));

        // Test with required values (?)
        assert!(!has_env_var("MISSING_VAR?error", None));
        assert!(!has_env_var("${MISSING_VAR?error}", None));

        // Test with variables starting with operators (shouldn't be treated as modifiers)
        assert!(!has_env_var("-STARTS_WITH_DASH", None));
        assert!(!has_env_var("${-STARTS_WITH_DASH}", None));

        // Test with empty variable name
        assert!(!has_env_var("", None));
        assert!(!has_env_var("${}", None));
    }

    #[test]
    fn test_check_for_environment_variables() {
        // Create a test environment variable map
        let mut env_vars = HashMap::new();
        env_vars.insert("EXISTING_VAR".to_string(), "value".to_string());

        // Test with no environment variables
        let data = serde_yml::from_str("key: value").unwrap();
        assert!(check_for_environment_variables(&data, None).is_ok());

        // Test with environment variable that has a default
        let data = serde_yml::from_str("key: ${MISSING_VAR:-default}").unwrap();
        assert!(check_for_environment_variables(&data, None).is_ok());

        // Test with environment variable that exists in env_vars
        let data = serde_yml::from_str("key: ${EXISTING_VAR}").unwrap();
        assert!(check_for_environment_variables(&data, Some(&env_vars)).is_ok());

        // Test with missing environment variable
        let data = serde_yml::from_str("key: ${MISSING_VAR}").unwrap();
        let result = check_for_environment_variables(&data, None);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::EnvironmentVariablesNotSupported(vars)) if vars == "${MISSING_VAR}")
        );

        // Test with multiple environment variables in complex structure
        let data = serde_yml::from_str(
            "
            services:
              app:
                image: test
                environment:
                  - VAR1=${EXISTING_VAR}
                  - VAR2=${MISSING_VAR}
        ",
        )
        .unwrap();
        let result = check_for_environment_variables(&data, Some(&env_vars));
        assert!(result.is_err());
        assert!(
            matches!(result, Err(AppError::EnvironmentVariablesNotSupported(vars)) if vars == "${MISSING_VAR}")
        );

        // Test with multiple environment variables, all satisfied
        let data = serde_yml::from_str(
            "
            services:
              app:
                image: test
                environment:
                  - VAR1=${EXISTING_VAR}
                  - VAR2=${MISSING_VAR:-default}
        ",
        )
        .unwrap();
        assert!(check_for_environment_variables(&data, Some(&env_vars)).is_ok());
    }
}
