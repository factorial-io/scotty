use bollard_stubs::models::{
    NetworkConnectRequest, NetworkCreateRequest, NetworkDisconnectRequest,
};
use tracing::{error, info, instrument, warn};

use crate::docker::loadbalancer::{server_status, traefik_target};

use super::context::Context;

/// Resolves the per-app proxy network name and the Traefik container to
/// connect to, or `None` when load balancing is not Traefik (e.g. HAProxy),
/// in which case the network handlers are a no-op.
fn proxy_network_target(context: &Context) -> Option<(String, String)> {
    let target = traefik_target(&context.app_state.settings)?;
    let network = target.network_for(&context.app_data.name);
    Some((network, target.container))
}

/// Creates the app's dedicated proxy network (if missing) and connects the
/// Traefik container to it. Runs before `docker compose up`, because the
/// override declares the network as external and Compose fails if it does not
/// already exist. All operations are idempotent so retries are safe.
#[instrument(skip_all, fields(app = %context.app_data.name))]
pub async fn ensure_app_network(context: &Context) -> anyhow::Result<()> {
    {
        let Some((network, container)) = proxy_network_target(context) else {
            return Ok(());
        };
        let docker = &context.app_state.docker;

        // Create the network. Ignore 409 (already exists) for idempotency.
        let mut labels = std::collections::HashMap::new();
        labels.insert("scotty.managed".to_string(), "true".to_string());
        labels.insert("scotty.app".to_string(), context.app_data.name.clone());
        // Defaults give a local bridge network with Docker-assigned IPAM, which
        // is what Traefik's container-to-container routing expects here.
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
                // 404 covers both "Traefik container missing" and "network
                // missing" (e.g. a concurrent destroy removed the network we
                // just created). We proceed best-effort; if it is the network
                // that is gone, the subsequent `compose up` surfaces it as a
                // hard failure.
                warn!(
                    "connect_network returned 404 for Traefik '{}' on network {} (container or network missing); app may not be routable",
                    container, network
                );
            }
            Err(e) => return Err(anyhow::Error::from(e)),
        }

        Ok(())
    }
}

/// Disconnects Traefik from the app's proxy network and removes the network.
/// Runs after `docker compose down`/`rm`, because Docker refuses to remove a
/// network while an endpoint (Traefik) is still attached. All operations are
/// idempotent and best-effort: teardown never fails the surrounding task.
#[instrument(skip_all, fields(app = %context.app_data.name))]
pub async fn teardown_app_network(context: &Context) -> anyhow::Result<()> {
    {
        let Some((network, container)) = proxy_network_target(context) else {
            return Ok(());
        };
        let docker = &context.app_state.docker;

        // Disconnect Traefik. `force` is intentional: the app's containers are
        // already down at teardown time, so there is no in-flight request to
        // disrupt, and force lets the disconnect succeed regardless of endpoint
        // state. Tolerate "not found / not connected", whose status is
        // version-dependent (403/404/409), so teardown stays idempotent.
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
            Err(e) if matches!(server_status(&e), Some(403 | 404 | 409)) => {
                // Benign "already disconnected / not connected" case. Log it so
                // that, if remove_network then reports a lingering endpoint, the
                // teardown trace is complete rather than a lone unexplained warning.
                info!(
                    "Traefik ({}) already disconnected from {} (status {:?})",
                    container,
                    network,
                    server_status(&e)
                );
            }
            Err(e) => warn!("Failed to disconnect Traefik from {}: {}", network, e),
        }

        // Remove the network. Ignore 404 (already gone); a 409 means other
        // endpoints are still attached, in which case we leave it in place.
        match docker.remove_network(&network).await {
            Ok(_) => info!("Removed proxy network {}", network),
            Err(e) if server_status(&e) == Some(404) => {}
            // The network could not be removed and is now leaked (e.g. Traefik
            // is still attached, which usually traces back to the disconnect
            // warning logged just above). Surface at error! with the name so an
            // operator can clean it up; it is also reclaimable on the next purge.
            Err(e) => error!(
                "Leaked proxy network {} (removal failed; Traefik may still be attached, see preceding disconnect log): {}",
                network, e
            ),
        }

        Ok(())
    }
}
