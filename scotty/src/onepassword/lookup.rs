use std::collections::HashMap;

use tracing::{debug, error};

use crate::settings::config::Settings;

use super::api::get_item;
use super::env_substitution::process_env_vars;

pub async fn resolve_environment_variables(
    settings: &Settings,
    env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    // First pass - resolve 1Password references
    let mut onepassword_resolved = HashMap::new();
    for (key, value) in env {
        let resolved_value = if value.starts_with("op://") {
            match lookup_password(settings, value).await {
                Ok(resolved_value) => resolved_value,
                Err(e) => {
                    error!("Failed to resolve password for {}: {}", key, e);
                    value.clone()
                }
            }
        } else {
            value.clone()
        };
        onepassword_resolved.insert(key.clone(), resolved_value);
    }

    // Second pass - resolve environment variable substitutions
    for (key, value) in onepassword_resolved.iter() {
        match process_env_vars(value, &onepassword_resolved) {
            Ok(processed_value) => {
                if processed_value != *value {
                    debug!("Processed env vars in value for {}", key);
                }
                resolved.insert(key.clone(), processed_value);
            }
            Err(e) => {
                error!("Error processing env vars for {}: {}", key, e);
                resolved.insert(key.clone(), value.clone());
            }
        }
    }

    resolved
}

async fn lookup_password(settings: &Settings, op_uri: &str) -> anyhow::Result<String> {
    // Remove "op://" prefix
    let parts: Vec<&str> = op_uri
        .strip_prefix("op://")
        .ok_or_else(|| anyhow::anyhow!("Invalid op:// URI"))?
        .split('/')
        .collect();

    // Check for required minimum parts
    if parts.len() < 3 {
        return Err(anyhow::anyhow!(
            "Invalid op:// URI format - requires at least token_name/vault_id/item_id"
        ));
    }

    let token_name = parts[0];
    let vault_id = parts[1];
    let item_id = parts[2];
    let (section_name, field_id) = if parts.len() == 5 {
        (Some(parts[3]), Some(parts[4]))
    } else {
        (None, parts.get(3).copied())
    };

    let onepassword_settings = match settings.onepassword.get(token_name) {
        Some(s) => s,
        None => {
            return Err(anyhow::anyhow!(
                "Failed to get OnePassword settings for token_name : {}",
                token_name
            ))
        }
    };

    let item = get_item(onepassword_settings, vault_id, item_id).await?;

    let result = match field_id {
        Some(f) => item.get_field_value(f, section_name),
        None => item.get_password(),
    };

    match result {
        Some(v) => Ok(v.to_string()),
        None => Err(anyhow::anyhow!(
            "Failed to get field value for field_id : {:?}",
            field_id
        )),
    }
}
#[cfg(test)]
mod tests {
    use crate::settings::config::OnePasswordSettings;

    use super::*;
    use maplit::hashmap;
    use std::collections::HashMap;
    use tokio;

    #[tokio::test]
    #[ignore]
    async fn test_resolve_environment_variables() {
        let mut env = HashMap::new();
        env.insert("KEY1".to_string(), "value1".to_string());
        env.insert(
            "USERNAME".to_string(),
            "op://factorial/n33i6edy47edsntxuj3a7lgiz4/ida4izoksx4mwdpvt7wbbq6d7y/username"
                .to_string(),
        );
        env.insert(
            "PASSWORD".to_string(),
            "op://factorial/n33i6edy47edsntxuj3a7lgiz4/ida4izoksx4mwdpvt7wbbq6d7y".to_string(),
        );
        env.insert(
            "SECTION_A_SERVER".to_string(),
            "op://factorial/n33i6edy47edsntxuj3a7lgiz4/ida4izoksx4mwdpvt7wbbq6d7y/Section A/server"
                .to_string(),
        );
        env.insert(
            "SECTION_A_PASSWORD".to_string(),
            "op://factorial/n33i6edy47edsntxuj3a7lgiz4/ida4izoksx4mwdpvt7wbbq6d7y/Section A/password".to_string(),
        );

        let onepassword_settings = OnePasswordSettings {
            jwt_token: std::env::var("SCOTTY_OP_JWT_TEST_TOKEN")
                .expect("SCOTTY_OP_JWT_TEST_TOKEN not set"),
            server: "https://vault.factorial.io".to_string(),
        };

        let settings = Settings {
            onepassword: hashmap! { "factorial".to_string() => onepassword_settings },
            ..Settings::default()
        };

        let resolved = resolve_environment_variables(&settings, &env).await;

        assert_eq!(resolved.get("KEY1").unwrap(), "value1");
        assert_eq!(resolved.get("USERNAME").unwrap(), "scotty@factorial.io");
        assert_eq!(resolved.get("PASSWORD").unwrap(), "my-little-secret");
        assert_eq!(
            resolved.get("SECTION_A_SERVER").unwrap(),
            "https://scotty.test.url"
        );
        assert_eq!(resolved.get("SECTION_A_PASSWORD").unwrap(), "second-secret");
    }

    #[tokio::test]
    async fn test_env_var_substitution() {
        let mut env = HashMap::new();
        env.insert("DATABASE_USER".to_string(), "db_user".to_string());
        env.insert("DATABASE_HOST".to_string(), "localhost".to_string());
        env.insert("DATABASE_PORT".to_string(), "5432".to_string());
        env.insert("DATABASE_PASSWORD".to_string(), "password123".to_string());
        env.insert("EMPTY".to_string(), "".to_string());

        // Test various substitution patterns
        env.insert("CONNECTION_STRING".to_string(), 
            "postgresql://${DATABASE_USER}:${DATABASE_PASSWORD}@${DATABASE_HOST}:${DATABASE_PORT}/mydb".to_string());
        env.insert(
            "BACKUP_HOST".to_string(),
            "${BACKUP_HOST_OVERRIDE:-${DATABASE_HOST}}".to_string(),
        );
        env.insert(
            "LOG_LEVEL".to_string(),
            "${DEBUG_MODE+debug}${DEBUG_MODE:-info}".to_string(),
        );
        env.insert(
            "REQUIRED_VAR".to_string(),
            "${CRITICAL_CONFIG?Missing critical configuration}".to_string(),
        );

        let settings = Settings::default();
        let resolved = resolve_environment_variables(&settings, &env).await;

        // Check the connection string with multiple variable substitutions
        assert_eq!(
            resolved.get("CONNECTION_STRING").unwrap(),
            "postgresql://db_user:password123@localhost:5432/mydb"
        );

        // Check nested variable substitution with default
        assert_eq!(resolved.get("BACKUP_HOST").unwrap(), "localhost");

        // Check conditional with default
        assert_eq!(resolved.get("LOG_LEVEL").unwrap(), "info");

        // The error message should be part of the value rather than causing a hard error
        assert!(resolved.get("REQUIRED_VAR").unwrap().contains("ERROR"));
    }
}
