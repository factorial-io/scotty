use std::sync::Arc;

use scotty_core::{apps::app_data::AppSettings, settings::loadbalancer::LoadBalancerType};
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    docker::loadbalancer::{factory::LoadBalancerFactory, types::DockerComposeConfig},
    onepassword::lookup::resolve_environment_variables,
    settings::config::Settings,
    state_machine::StateHandler,
};

use super::context::Context;

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
    resolved_environment: &std::collections::HashMap<String, String>,
) -> anyhow::Result<DockerComposeConfig> {
    let lb = LoadBalancerFactory::create(load_balancer_type);
    let docker_compose_override =
        lb.get_docker_compose_override(global_settings, app_name, settings, resolved_environment)?;
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
        let docker_compose_override = get_docker_compose_override(
            &self.load_balancer_type,
            &context.app_state.settings,
            &context.app_data.name,
            &self.settings,
            &resolved_environment,
        )?;
        let path = root_directory.join("docker-compose.override.yml");
        info!("Saving docker-compose.override.yml to {}", path.display());
        let yaml = serde_yml::to_string(&docker_compose_override)?;
        tokio::fs::write(&path, yaml).await?;

        Ok(self.next_state.clone())
    }
}
