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
///
/// `app_name` is expected to be a slug (Scotty slugifies app names on
/// create/adopt), which keeps the result within Docker's allowed network-name
/// character set. The join is not injective if `base_network` itself contains
/// `--` (e.g. base `proxy--region` + app `foo` yields the same name as base
/// `proxy` + app `region--foo`); the default base `proxy` has no dashes, so in
/// practice the network name is determined by the app name.
pub fn app_proxy_network_name(base_network: &str, app_name: &str) -> String {
    format!("{base_network}--{app_name}")
}
