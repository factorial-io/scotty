use anyhow;
use dotenvy;
use scotty_core::{
    apps::app_data::{AppTtl, ServicePortMapping},
    apps::create_app_request::CustomDomainMapping,
    notification_types::{GitlabContext, MattermostContext, NotificationReceiver, WebhookContext},
};

pub fn parse_service_ids(s: &str) -> Result<NotificationReceiver, String> {
    let parts: Vec<&str> = s.split("://").collect();

    if parts.len() < 2 {
        return Err("Invalid service ID format".to_string());
    }
    let service_type = parts[0];

    let parts = parts[1].split("/").collect::<Vec<&str>>();
    if parts.is_empty() {
        return Err("Invalid service ID format".to_string());
    }
    let service_id = parts[0];

    match service_type {
        "log" => Ok(NotificationReceiver::Log),
        "webhook" => {
            if parts.len() != 1 {
                return Err("Invalid service ID format for webhook".to_string());
            }
            Ok(NotificationReceiver::Webhook(WebhookContext {
                service_id: service_id.to_string(),
            }))
        }
        "mattermost" => {
            if parts.len() != 2 {
                return Err("Invalid service ID format for mattermost".to_string());
            }
            let channel = parts[1];
            Ok(NotificationReceiver::Mattermost(MattermostContext {
                service_id: service_id.to_string(),
                channel: channel.to_string(),
            }))
        }
        "gitlab" => {
            if parts.len() < 3 {
                return Err("Invalid service ID format for gitlab".to_string());
            }
            let project_id = parts[1..parts.len() - 1].join("/").to_string();
            let mr_id = parts.last().unwrap().parse::<u64>().unwrap();
            Ok(NotificationReceiver::Gitlab(GitlabContext {
                service_id: service_id.to_string(),
                project_id,
                mr_id,
            }))
        }
        _ => Err(format!(
            "Unknown service type {service_type}, allowed values are log, mattermost, webhook and gitlab"
        )),
    }
}

pub fn parse_app_ttl(s: &str) -> Result<AppTtl, String> {
    if s.eq_ignore_ascii_case("forever") {
        return Ok(AppTtl::Forever);
    }
    if let Some(days) = s.strip_suffix("d") {
        if let Ok(num_days) = days.parse::<u32>() {
            return Ok(AppTtl::Days(num_days));
        }
    }
    if let Some(hours) = s.strip_suffix("h") {
        if let Ok(num_hours) = hours.parse::<u32>() {
            return Ok(AppTtl::Hours(num_hours)); // Assuming AppTtl has a variant called `Hours`
        }
    }
    Err(format!("Invalid TTL format: {s}"))
}

pub fn parse_folder_containing_docker_compose(s: &str) -> Result<String, String> {
    let path = std::path::Path::new(s);
    if path.is_dir() {
        scotty_core::utils::compose::find_config_file_in_dir(path)
            .map(|p| p.to_string_lossy().to_string())
            .ok_or_else(|| "Folder does not contain a Docker Compose standard config file, such as docker-compose.yaml or compose.yaml.".to_string())
    } else {
        Err("No Docker Compose config file found, path is not a directory".to_string())
    }
}

pub fn parse_basic_auth(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid basic auth format, should be user:password".to_string());
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

pub fn parse_custom_domain_mapping(s: &str) -> Result<CustomDomainMapping, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid custom domain format, should be domain:service".to_string());
    }
    Ok(CustomDomainMapping {
        domain: parts[0].to_string(),
        service: parts[1].to_string(),
    })
}

pub fn parse_service_ports(s: &str) -> Result<ServicePortMapping, String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Invalid service port format, should be service:port".to_string());
    }
    let port = parts[1]
        .parse::<u32>()
        .map_err(|_| "Invalid port number".to_string())?;
    Ok(ServicePortMapping {
        service: parts[0].to_string(),
        port,
        domains: vec![],
    })
}

pub fn parse_env_vars(s: &str) -> Result<(String, String), String> {
    match s.find('=') {
        Some(idx) => {
            let key = &s[..idx];
            let value = &s[idx + 1..];
            Ok((key.to_string(), value.to_string()))
        }
        None => Err("Invalid env var format, should be key=value".to_string()),
    }
}

pub fn parse_env_file(file_path: &str) -> anyhow::Result<Vec<(String, String)>> {
    // Use dotenvy to parse the .env file
    let env_vars = dotenvy::from_path_iter(file_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse env file: {}", e))?;

    // Convert to Vec<(String, String)>
    let env_vars: Vec<(String, String)> = env_vars
        .filter_map(|result| {
            result
                .map_err(|e| {
                    eprintln!("Warning: {e}");
                    e
                })
                .ok()
        })
        .collect();

    Ok(env_vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_vars() {
        // Test normal key-value pairs
        let result = parse_env_vars("KEY=value");
        assert!(result.is_ok());
        let (key, value) = result.unwrap();
        assert_eq!(key, "KEY");
        assert_eq!(value, "value");

        // Test value containing equals sign
        let result = parse_env_vars("KEY=value=with=equals");
        assert!(result.is_ok());
        let (key, value) = result.unwrap();
        assert_eq!(key, "KEY");
        assert_eq!(value, "value=with=equals");

        // Test empty value
        let result = parse_env_vars("KEY=");
        assert!(result.is_ok());
        let (key, value) = result.unwrap();
        assert_eq!(key, "KEY");
        assert_eq!(value, "");

        // Test error case - no equals sign
        let result = parse_env_vars("INVALID_FORMAT");
        assert!(result.is_err());
    }
}
