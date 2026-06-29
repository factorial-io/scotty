pub mod factory;
pub mod haproxy;
pub mod traefik;
pub mod types;

/// Computes the name of the per-app Traefik proxy network.
///
/// Each app gets its own dedicated external network (derived from the
/// configured base network name plus the app name) instead of all apps
/// sharing one global network. This keeps each app's Docker DNS namespace
/// isolated so service names (e.g. `nginx`) can never collide across apps.
///
/// Must stay in sync between the compose-override generation
/// (`traefik.rs`) and the network lifecycle handlers, which both build the
/// name from the same inputs.
pub fn app_proxy_network_name(base_network: &str, app_name: &str) -> String {
    format!("{base_network}--{app_name}")
}
