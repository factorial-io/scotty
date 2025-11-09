use std::sync::Arc;

use scotty_core::{
    apps::app_data::AppSettings, settings::loadbalancer::LoadBalancerType,
    utils::secret::SecretHashMap,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    docker::loadbalancer::{factory::LoadBalancerFactory, types::DockerComposeConfig},
    onepassword::lookup::resolve_environment_variables,
    settings::config::Settings,
    state_machine::StateHandler,
};

use super::context::Context;

/// Reads service names from a compose file
async fn get_service_names_from_compose(
    compose_path: &std::path::Path,
) -> anyhow::Result<Vec<String>> {
    let content = tokio::fs::read_to_string(compose_path).await?;
    let yaml: serde_norway::Value = serde_norway::from_str(&content)?;

    let mut service_names = Vec::new();
    if let Some(services) = yaml.get("services") {
        if let Some(services_map) = services.as_mapping() {
            for (key, _) in services_map {
                if let Some(service_name) = key.as_str() {
                    service_names.push(service_name.to_string());
                }
            }
        }
    }

    Ok(service_names)
}

#[derive(Debug)]
pub struct CreateLoadBalancerConfig<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
    pub load_balancer_type: LoadBalancerType,
    pub settings: AppSettings,
}

fn get_docker_compose_override(
    load_balancer_type: &LoadBalancerType,
    global_settings: &Settings,
    app_name: &str,
    settings: &AppSettings,
    resolved_environment: &SecretHashMap,
    all_services: &[String],
) -> anyhow::Result<DockerComposeConfig> {
    let lb = LoadBalancerFactory::create(load_balancer_type);
    // Expose secrets only here, at the point where we need plain values for YAML generation
    let exposed_environment = resolved_environment.expose_all();
    let docker_compose_override = lb.get_docker_compose_override(
        global_settings,
        app_name,
        settings,
        &exposed_environment,
        all_services,
    )?;
    Ok(docker_compose_override)
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for CreateLoadBalancerConfig<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let root_directory = std::path::PathBuf::from(&context.app_data.root_directory);
        let resolved_environment =
            resolve_environment_variables(&context.app_state.settings, &self.settings.environment)
                .await;

        // Find and read all service names from the compose file
        let compose_path = scotty_core::utils::compose::find_config_file_in_dir(&root_directory)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Folder {} does not contain a Docker Compose standard config file, such as docker-compose.yaml or compose.yaml.",
                    root_directory.display()
                )
            })?;

        let all_services = get_service_names_from_compose(&compose_path).await?;

        // Pass SecretHashMap - secrets will be exposed inside get_docker_compose_override
        let docker_compose_override = get_docker_compose_override(
            &self.load_balancer_type,
            &context.app_state.settings,
            &context.app_data.name,
            &self.settings,
            &resolved_environment,
            &all_services,
        )?;

        let override_file = scotty_core::utils::compose::get_override_file(&compose_path)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unable to determine override file path from compose file: {}",
                    compose_path.display()
                )
            })?;

        info!("Saving override file to {}", override_file.display());
        let yaml = serde_norway::to_string(&docker_compose_override)?;
        tokio::fs::write(&override_file, yaml).await?;

        Ok(self.next_state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scotty_core::apps::app_data::{AppSettings, ServicePortMapping};
    use scotty_core::settings::loadbalancer::{LoadBalancerType, TraefikSettings};
    use std::collections::HashMap;

    #[test]
    fn test_docker_compose_override_contains_unmasked_secrets() {
        // Create settings with environment variables containing secrets
        let mut environment = HashMap::new();
        environment.insert(
            "DATABASE_PASSWORD".to_string(),
            "super-secret-password-123".to_string(),
        );
        environment.insert("API_KEY".to_string(), "sk-1234567890abcdef".to_string());
        environment.insert("SECRET_TOKEN".to_string(), "jwt-token-xyz-789".to_string());
        environment.insert("NORMAL_VAR".to_string(), "not-a-secret".to_string());
        let environment = SecretHashMap::from_hashmap(environment);

        let app_settings = AppSettings {
            domain: "example.com".to_string(),
            public_services: vec![ServicePortMapping {
                service: "web".to_string(),
                port: 8080,
                domains: vec![],
            }],
            environment: environment.clone(),
            ..Default::default()
        };

        let global_settings = Settings {
            traefik: TraefikSettings::new(false, "proxy".into(), None, vec![]),
            ..Default::default()
        };

        let all_services = vec!["web".to_string(), "db".to_string()];

        // Generate docker-compose override
        let override_config = get_docker_compose_override(
            &LoadBalancerType::Traefik,
            &global_settings,
            "test-app",
            &app_settings,
            &environment,
            &all_services,
        )
        .unwrap();

        // Serialize to YAML (simulating what gets written to disk)
        let yaml_output = serde_norway::to_string(&override_config).unwrap();

        // Verify that ACTUAL secret values are in the YAML, not masked versions
        // This is REQUIRED for the containers to work properly
        assert!(
            yaml_output.contains("super-secret-password-123"),
            "DATABASE_PASSWORD should contain the real password, not masked. Found:\n{}",
            yaml_output
        );
        assert!(
            yaml_output.contains("sk-1234567890abcdef"),
            "API_KEY should contain the real key, not masked. Found:\n{}",
            yaml_output
        );
        assert!(
            yaml_output.contains("jwt-token-xyz-789"),
            "SECRET_TOKEN should contain the real token, not masked. Found:\n{}",
            yaml_output
        );

        // Verify that secrets are NOT masked (these would be the masked versions)
        assert!(
            !yaml_output.contains("***************123"),
            "DATABASE_PASSWORD should NOT be masked in docker-compose.override.yml"
        );
        assert!(
            !yaml_output.contains("**************cdef"),
            "API_KEY should NOT be masked in docker-compose.override.yml"
        );
        assert!(
            !yaml_output.contains("***-*****-***-789"),
            "SECRET_TOKEN should NOT be masked in docker-compose.override.yml"
        );

        // Both web and db services should have the unmasked environment variables
        let web_service = override_config.services.get("web").unwrap();
        let web_env = web_service.environment.as_ref().unwrap();
        assert_eq!(
            web_env.get("DATABASE_PASSWORD").unwrap(),
            "super-secret-password-123"
        );
        assert_eq!(web_env.get("API_KEY").unwrap(), "sk-1234567890abcdef");
        assert_eq!(web_env.get("SECRET_TOKEN").unwrap(), "jwt-token-xyz-789");

        let db_service = override_config.services.get("db").unwrap();
        let db_env = db_service.environment.as_ref().unwrap();
        assert_eq!(
            db_env.get("DATABASE_PASSWORD").unwrap(),
            "super-secret-password-123"
        );
        assert_eq!(db_env.get("API_KEY").unwrap(), "sk-1234567890abcdef");
        assert_eq!(db_env.get("SECRET_TOKEN").unwrap(), "jwt-token-xyz-789");
    }
}
