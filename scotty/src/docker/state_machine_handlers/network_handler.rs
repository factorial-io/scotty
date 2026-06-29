use std::sync::Arc;

use bollard::errors::Error as BollardError;
use bollard_stubs::models::{
    NetworkConnectRequest, NetworkCreateRequest, NetworkDisconnectRequest,
};
use scotty_core::settings::loadbalancer::LoadBalancerType;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

use crate::docker::loadbalancer::app_proxy_network_name;
use crate::state_machine::StateHandler;

use super::context::Context;

/// Returns the HTTP status code for a Docker daemon error, if any.
fn server_status(err: &BollardError) -> Option<u16> {
    match err {
        BollardError::DockerResponseServerError { status_code, .. } => Some(*status_code),
        _ => None,
    }
}

/// Resolves the per-app proxy network name and the Traefik container to
/// connect to, or `None` when load balancing is not Traefik (e.g. HAProxy),
/// in which case the network handlers are a no-op.
fn proxy_network_target(context: &Context) -> Option<(String, String)> {
    let settings = &context.app_state.settings;
    if settings.load_balancer_type != LoadBalancerType::Traefik {
        return None;
    }
    let network = app_proxy_network_name(&settings.traefik.network, &context.app_data.name);
    let container = settings.traefik.container_name.clone();
    Some((network, container))
}

/// Creates the app's dedicated proxy network (if missing) and connects the
/// Traefik container to it. Runs before `docker compose up`, because the
/// override declares the network as external and Compose fails if it does not
/// already exist. All operations are idempotent so retries are safe.
#[derive(Debug)]
pub struct EnsureAppNetworkHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for EnsureAppNetworkHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let Some((network, container)) = proxy_network_target(&context) else {
            return Ok(self.next_state.clone());
        };
        let docker = &context.app_state.docker;

        // Create the network. Ignore 409 (already exists) for idempotency.
        let mut labels = std::collections::HashMap::new();
        labels.insert("scotty.managed".to_string(), "true".to_string());
        labels.insert("scotty.app".to_string(), context.app_data.name.clone());
        match docker
            .create_network(NetworkCreateRequest {
                name: network.clone(),
                labels: Some(labels),
                ..Default::default()
            })
            .await
        {
            Ok(_) => info!("Created proxy network {}", network),
            Err(e) if server_status(&e) == Some(409) => {
                info!("Proxy network {} already exists", network);
            }
            Err(e) => return Err(anyhow::Error::from(e)),
        }

        // Connect Traefik to the network. Ignore 403 (already connected). A
        // missing Traefik container (404) is logged but not fatal: the app
        // still runs, it just is not routable until Traefik is available.
        match docker
            .connect_network(
                &network,
                NetworkConnectRequest {
                    container: container.clone(),
                    endpoint_config: None,
                },
            )
            .await
        {
            Ok(_) => info!("Connected Traefik ({}) to network {}", container, network),
            // Already connected. The exact status is version-dependent: older
            // daemons raise a libnetwork "endpoint already exists" ForbiddenError
            // (403), newer ones a Conflict (409). Treat both as benign so the
            // handler is idempotent across Docker versions.
            Err(e) if matches!(server_status(&e), Some(403 | 409)) => {
                info!("Traefik ({}) already connected to {}", container, network);
            }
            Err(e) if server_status(&e) == Some(404) => {
                warn!(
                    "Traefik container '{}' not found; app on network {} will not be routable until it is connected",
                    container, network
                );
            }
            Err(e) => return Err(anyhow::Error::from(e)),
        }

        Ok(self.next_state.clone())
    }
}

/// Disconnects Traefik from the app's proxy network and removes the network.
/// Runs after `docker compose down`/`rm`, because Docker refuses to remove a
/// network while an endpoint (Traefik) is still attached. All operations are
/// idempotent and best-effort: teardown never fails the surrounding task.
#[derive(Debug)]
pub struct TeardownAppNetworkHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    pub next_state: S,
}

#[async_trait::async_trait]
impl<S> StateHandler<S, Context> for TeardownAppNetworkHandler<S>
where
    S: Send + Sync + Clone + std::fmt::Debug,
{
    #[instrument(skip(context))]
    async fn transition(&self, _from: &S, context: Arc<RwLock<Context>>) -> anyhow::Result<S> {
        let context = context.read().await;
        let Some((network, container)) = proxy_network_target(&context) else {
            return Ok(self.next_state.clone());
        };
        let docker = &context.app_state.docker;

        // Disconnect Traefik. Ignore "not found / not connected" (404).
        match docker
            .disconnect_network(
                &network,
                NetworkDisconnectRequest {
                    container: container.clone(),
                    force: Some(true),
                },
            )
            .await
        {
            Ok(_) => info!(
                "Disconnected Traefik ({}) from network {}",
                container, network
            ),
            Err(e) if server_status(&e) == Some(404) => {}
            Err(e) => warn!("Failed to disconnect Traefik from {}: {}", network, e),
        }

        // Remove the network. Ignore 404 (already gone); a 409 means other
        // endpoints are still attached, in which case we leave it in place.
        match docker.remove_network(&network).await {
            Ok(_) => info!("Removed proxy network {}", network),
            Err(e) if server_status(&e) == Some(404) => {}
            Err(e) => warn!("Could not remove proxy network {}: {}", network, e),
        }

        Ok(self.next_state.clone())
    }
}
