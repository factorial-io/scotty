use anyhow::anyhow;
use regex::Regex;
use std::collections::HashMap;
use std::env;

/// Processes a string containing environment variable references and substitutes them
/// according to Bash-like syntax rules.
///
/// Supports the following patterns:
/// - $VAR or ${VAR} - Basic variable substitution
/// - ${VAR:-default} - Use default value if VAR is unset or empty
/// - ${VAR-default} - Use default value if VAR is unset
/// - ${VAR:?error} - Display error if VAR is unset or empty
/// - ${VAR?error} - Display error if VAR is unset
/// - ${VAR:+replacement} - Use replacement if VAR is set and not empty
/// - ${VAR+replacement} - Use replacement if VAR is set
///
/// # Arguments
///
/// * `input` - String containing environment variable references
/// * `env_vars` - HashMap of environment variables for substitution
///
/// # Returns
///
/// The string with all environment variable references processed
pub fn process_env_vars(input: &str, env_vars: &HashMap<String, String>) -> anyhow::Result<String> {
    // Loop until no more substitutions can be made
    let mut result = input.to_string();
    let mut last_result = String::new();

    let braces_regex = Regex::new(r"\$\{([^{}]+?)\}").unwrap();
    let simple_regex = Regex::new(r"\$(\w+)").unwrap();

    // Process substitutions until we reach a fixed point (no more changes)
    while result != last_result {
        last_result = result.clone();

        // Process ${VAR} syntax first (with all modifiers)
        result = braces_regex
            .replace_all(&last_result, |caps: &regex::Captures| {
                process_var_with_braces(caps.get(1).unwrap().as_str(), env_vars)
                    .unwrap_or_else(|e| format!("ERROR: {}", e))
            })
            .to_string();

        // Then process simple $VAR syntax
        result = simple_regex
            .replace_all(&result, |caps: &regex::Captures| {
                let var_name = caps.get(1).unwrap().as_str();
                match get_var_value(var_name, env_vars) {
                    Some(value) => value,
                    None => format!("${}", var_name), // Leave unchanged if not found
                }
            })
            .to_string();
    }

    Ok(result)
}

/// Process variable with braces syntax like ${VAR}, ${VAR:-default}, etc.
fn process_var_with_braces(
    var_expr: &str,
    env_vars: &HashMap<String, String>,
) -> anyhow::Result<String> {
    // Check for operator patterns
    if let Some(idx) = var_expr.find(":-") {
        // ${VAR:-default} - Use default if var is unset or empty
        let var_name = &var_expr[..idx];
        let default_value = &var_expr[idx + 2..];
        match get_var_value(var_name, env_vars) {
            Some(value) if !value.is_empty() => Ok(value),
            _ => Ok(default_value.to_string()),
        }
    } else if let Some(idx) = var_expr.find("-") {
        // ${VAR-default} - Use default if var is unset
        let var_name = &var_expr[..idx];
        let default_value = &var_expr[idx + 1..];
        match get_var_value(var_name, env_vars) {
            Some(value) => return Ok(value),
            None => return Ok(default_value.to_string()),
        }
    } else if let Some(idx) = var_expr.find(":?") {
        // ${VAR:?error} - Display error if var is unset or empty
        let var_name = &var_expr[..idx];
        let error_msg = &var_expr[idx + 2..];
        match get_var_value(var_name, env_vars) {
            Some(value) if !value.is_empty() => return Ok(value),
            _ => {
                return Err(anyhow!(
                    "Variable '{}' is unset or empty: {}",
                    var_name,
                    error_msg
                ))
            }
        }
    } else if let Some(idx) = var_expr.find("?") {
        // ${VAR?error} - Display error if var is unset
        let var_name = &var_expr[..idx];
        let error_msg = &var_expr[idx + 1..];
        match get_var_value(var_name, env_vars) {
            Some(value) => return Ok(value),
            None => return Err(anyhow!("Variable '{}' is unset: {}", var_name, error_msg)),
        }
    } else if let Some(idx) = var_expr.find(":+") {
        // ${VAR:+replacement} - Use replacement if var is set and not empty
        let var_name = &var_expr[..idx];
        let replacement = &var_expr[idx + 2..];
        match get_var_value(var_name, env_vars) {
            Some(value) if !value.is_empty() => return Ok(replacement.to_string()),
            _ => return Ok("".to_string()),
        }
    } else if let Some(idx) = var_expr.find("+") {
        // ${VAR+replacement} - Use replacement if var is set
        let var_name = &var_expr[..idx];
        let replacement = &var_expr[idx + 1..];
        match get_var_value(var_name, env_vars) {
            Some(_) => return Ok(replacement.to_string()),
            None => return Ok("".to_string()),
        }
    } else {
        // Simple ${VAR} form
        match get_var_value(var_expr, env_vars) {
            Some(value) => return Ok(value),
            None => return Ok(format!("${{{}}}", var_expr)), // Leave unchanged if not found
        }
    }
}

/// Extract environment variable references from a string without substituting them
///
/// # Arguments
///
/// * `input` - String to extract environment variable references from
///
/// # Returns
///
/// A vector of environment variable references including the ${} or $ prefix
pub fn extract_env_vars(input: &str) -> Vec<String> {
    let mut variables = Vec::new();

    // Extract ${VAR} pattern variables
    let braces_regex = Regex::new(r"\$\{([^{}]+?)\}").unwrap();
    for captures in braces_regex.captures_iter(input) {
        if let Some(full_match) = captures.get(0) {
            variables.push(full_match.as_str().to_string());
        }
    }

    // Extract $VAR pattern variables
    let simple_regex = Regex::new(r"\$(\w+)").unwrap();
    for captures in simple_regex.captures_iter(input) {
        if let Some(full_match) = captures.get(0) {
            variables.push(full_match.as_str().to_string());
        }
    }

    variables
}

/// Gets a variable value from environment variables HashMap,
/// falling back to system environment variables if not found
fn get_var_value(var_name: &str, env_vars: &HashMap<String, String>) -> Option<String> {
    // First check in the provided env_vars
    if let Some(value) = env_vars.get(var_name) {
        return Some(value.clone());
    }

    // Fall back to system environment variables
    if let Ok(value) = env::var(var_name) {
        return Some(value);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::hashmap;

    #[test]
    fn test_basic_substitution() {
        let env_vars = hashmap! {
            "VAR1".to_string() => "value1".to_string(),
            "VAR2".to_string() => "value2".to_string(),
            "EMPTY".to_string() => "".to_string(),
        };

        assert_eq!(process_env_vars("$VAR1", &env_vars).unwrap(), "value1");
        assert_eq!(process_env_vars("${VAR2}", &env_vars).unwrap(), "value2");
        assert_eq!(
            process_env_vars("prefix-$VAR1-suffix", &env_vars).unwrap(),
            "prefix-value1-suffix"
        );
    }

    #[test]
    fn test_default_values() {
        let env_vars = hashmap! {
            "VAR1".to_string() => "value1".to_string(),
            "EMPTY".to_string() => "".to_string(),
        };

        // ${VAR:-default} - Use default if var is unset or empty
        assert_eq!(
            process_env_vars("${VAR1:-default}", &env_vars).unwrap(),
            "value1"
        );
        assert_eq!(
            process_env_vars("${EMPTY:-default}", &env_vars).unwrap(),
            "default"
        );
        assert_eq!(
            process_env_vars("${UNSET:-default}", &env_vars).unwrap(),
            "default"
        );

        // ${VAR-default} - Use default if var is unset
        assert_eq!(
            process_env_vars("${VAR1-default}", &env_vars).unwrap(),
            "value1"
        );
        assert_eq!(process_env_vars("${EMPTY-default}", &env_vars).unwrap(), "");
        assert_eq!(
            process_env_vars("${UNSET-default}", &env_vars).unwrap(),
            "default"
        );
    }

    #[test]
    fn test_error_messages() {
        let env_vars = hashmap! {
            "VAR1".to_string() => "value1".to_string(),
            "EMPTY".to_string() => "".to_string(),
        };

        // ${VAR:?error} - Error if var is unset or empty
        assert_eq!(
            process_env_vars("${VAR1:?error}", &env_vars).unwrap(),
            "value1"
        );

        // These now return the error as part of the string rather than as an Err result
        let empty_result = process_env_vars("${EMPTY:?error}", &env_vars).unwrap();
        assert!(empty_result.contains("ERROR"));
        assert!(empty_result.contains("is unset or empty"));

        let unset_result = process_env_vars("${UNSET:?error}", &env_vars).unwrap();
        assert!(unset_result.contains("ERROR"));
        assert!(unset_result.contains("is unset or empty"));

        // ${VAR?error} - Error if var is unset
        assert_eq!(
            process_env_vars("${VAR1?error}", &env_vars).unwrap(),
            "value1"
        );
        assert_eq!(process_env_vars("${EMPTY?error}", &env_vars).unwrap(), "");

        let unset_error_result = process_env_vars("${UNSET?error}", &env_vars).unwrap();
        assert!(unset_error_result.contains("ERROR"));
        assert!(unset_error_result.contains("is unset"));
    }

    #[test]
    fn test_replacement() {
        let env_vars = hashmap! {
            "VAR1".to_string() => "value1".to_string(),
            "EMPTY".to_string() => "".to_string(),
        };

        // ${VAR:+replacement} - Use replacement if var is set and not empty
        assert_eq!(
            process_env_vars("${VAR1:+replacement}", &env_vars).unwrap(),
            "replacement"
        );
        assert_eq!(
            process_env_vars("${EMPTY:+replacement}", &env_vars).unwrap(),
            ""
        );
        assert_eq!(
            process_env_vars("${UNSET:+replacement}", &env_vars).unwrap(),
            ""
        );

        // ${VAR+replacement} - Use replacement if var is set
        assert_eq!(
            process_env_vars("${VAR1+replacement}", &env_vars).unwrap(),
            "replacement"
        );
        assert_eq!(
            process_env_vars("${EMPTY+replacement}", &env_vars).unwrap(),
            "replacement"
        );
        assert_eq!(
            process_env_vars("${UNSET+replacement}", &env_vars).unwrap(),
            ""
        );
    }

    #[test]
    fn test_complex_expressions() {
        let env_vars = hashmap! {
            "USER".to_string() => "admin".to_string(),
            "HOST".to_string() => "example.com".to_string(),
            "PORT".to_string() => "8080".to_string(),
        };

        // Test basic substitution
        let input = "${USER}@${HOST}:${PORT:-80}/api/${SERVICE:-default}?token=${TOKEN-secret}";
        let expected = "admin@example.com:8080/api/default?token=secret";
        assert_eq!(process_env_vars(input, &env_vars).unwrap(), expected);

        // Test nested substitution
        let nested_input = "${OUTER:-${USER}@${HOST}}";
        let nested_expected = "admin@example.com";
        assert_eq!(
            process_env_vars(nested_input, &env_vars).unwrap(),
            nested_expected
        );

        // Test multiple levels of nesting
        let multi_nested = "${LEVEL1:-${LEVEL2:-${USER}}}";
        assert_eq!(process_env_vars(multi_nested, &env_vars).unwrap(), "admin");
    }

    #[test]
    fn test_extract_env_vars() {
        // Test with various environment variable patterns
        let test_str =
            "Connection: ${USER}:${PASSWORD} with $SIMPLE and ${VAR:-default} and ${OTHER-default}";
        let vars = extract_env_vars(test_str);

        assert_eq!(
            vars.len(),
            5,
            "Should extract all 5 environment variable references"
        );
        assert!(vars.contains(&"${USER}".to_string()));
        assert!(vars.contains(&"${PASSWORD}".to_string()));
        assert!(vars.contains(&"$SIMPLE".to_string()));
        assert!(vars.contains(&"${VAR:-default}".to_string()));
        assert!(vars.contains(&"${OTHER-default}".to_string()));

        // Test with more complex patterns
        let complex_str = "${VAR:+replacement} ${VAR+replacement} ${VAR:?error} ${VAR?error}";
        let complex_vars = extract_env_vars(complex_str);

        assert_eq!(
            complex_vars.len(),
            4,
            "Should extract all 4 complex environment variable patterns"
        );
        assert!(complex_vars.contains(&"${VAR:+replacement}".to_string()));
        assert!(complex_vars.contains(&"${VAR+replacement}".to_string()));
        assert!(complex_vars.contains(&"${VAR:?error}".to_string()));
        assert!(complex_vars.contains(&"${VAR?error}".to_string()));
    }
}
