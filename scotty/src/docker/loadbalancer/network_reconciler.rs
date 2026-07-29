//! Keeps Traefik's membership in the per-app proxy networks converged with the
//! set of deployed apps.
//!
//! Traefik is attached to an app's `<base>--<app>` network imperatively at
//! deploy time, and that attachment lives in container state rather than in
//! declared config. Recreating the Traefik container therefore silently drops
//! every app off the load balancer: containers stay up, TLS still terminates,
//! and requests just hang because they never reach a backend. This module turns
//! membership into a reconciled property instead: every running-app check (and,
//! when enabled, every start of the Traefik container) converges the actual
//! membership towards the desired one and records what it observed on each
//! `AppData`.
//!
//! Two properties are worth keeping in mind when changing this code:
//!
//! * The work set is derived from networks that **exist in Docker**, never from
//!   names computed for every app. That is what keeps legacy shared-network apps
//!   and never-deployed apps out of it, and it is why this module never creates
//!   a network — only the deploy path does.
//! * Pruning is guarded on "no non-Traefik endpoint attached", which makes the
//!   destructive half self-limiting: any app whose containers are up is
//!   protected automatically, even if it dropped out of the app list for an
//!   unrelated reason.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use std::time::Duration;

use bollard_stubs::models::{
    EventMessageTypeEnum, NetworkConnectRequest, NetworkDisconnectRequest,
};
use bollard_stubs::query_parameters::{
    EventsOptions, InspectContainerOptions, InspectNetworkOptions, ListNetworksOptions,
};
use futures_util::StreamExt;
use scotty_core::apps::app_data::{AppData, LoadBalancerConnectivity};
use scotty_core::apps::shared_app_list::AppDataVec;
use tokio::sync::Mutex;
use tracing::{debug, error, info, instrument, warn};

use crate::app_state::SharedAppState;

use super::{server_status, traefik_target, TraefikTarget};

/// Label marking a network as created and owned by Scotty. Only labelled
/// networks are ever removed.
const LABEL_MANAGED: &str = "scotty.managed";
/// Label naming the app a proxy network belongs to.
const LABEL_APP: &str = "scotty.app";

/// Serializes reconciliation passes.
///
/// Two triggers exist (the scheduled app check and the Docker event watcher), so
/// passes can collide. A scheduled pass waits for the lock; an event-driven pass
/// skips when it cannot take it, because the in-flight pass observes the same
/// Docker state anyway. This also keeps writes to the shared app list
/// serialized.
static PASS_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// What should happen to one existing proxy network.
#[derive(Debug, PartialEq, Eq)]
enum NetworkVerdict {
    /// Belongs to a discovered app with running containers: Traefik must be
    /// attached to it.
    Desired,
    /// Belongs to a discovered app that is not running, or is a network we do
    /// not own and cannot attribute: leave it exactly as it is.
    Ignore,
    /// Scotty-managed but belongs to no discovered app: a candidate for
    /// removal, subject to the endpoint guard in [`is_prunable`].
    OrphanCandidate,
}

/// A proxy network as seen in Docker.
#[derive(Debug, Clone)]
struct ProxyNetwork {
    name: String,
    /// Value of the `scotty.app` label, when present.
    app: Option<String>,
    /// Whether the `scotty.managed=true` label is present.
    managed: bool,
}

/// Result of one pass, reported as metrics.
#[derive(Debug, Default, PartialEq, Eq)]
struct PassOutcome {
    /// Running apps whose network Traefik was missing when the pass started.
    drifted: usize,
    /// Apps with public services that are still not reachable afterwards.
    unroutable: usize,
}

/// Whether the app declares public services, i.e. whether load-balancer
/// connectivity is something it needs at all.
///
/// Apps without a `.scotty.yml` (and therefore without settings) are still
/// reconciled — their network is attached if it exists — but they are never
/// counted as unroutable, because nothing declares that they should be
/// reachable.
fn has_public_services(app: &AppData) -> bool {
    app.settings
        .as_ref()
        .is_some_and(|s| !s.public_services.is_empty())
}

/// Whether any of the app's containers is up (or coming up).
///
/// Deliberately generous: `Created` and `Restarting` containers are ones Traefik
/// must be able to reach imminently, and an app in `Starting` is exactly the
/// state a partially-recovered host is in. Over-attaching is harmless; failing
/// to attach is the outage this module exists to prevent.
fn is_running(app: &AppData) -> bool {
    app.services.iter().any(|s| s.is_running())
}

/// Classifies one existing network against the discovered app list.
///
/// `app_networks` maps each discovered app's expected network name to the app's
/// name, so an unlabelled per-app network (created by a version that labelled
/// differently) is still recognised by name. Name matching is used for the
/// connect side only; [`NetworkVerdict::OrphanCandidate`] additionally requires
/// the managed label, because connecting is reversible and removing is not.
fn classify(
    network: &ProxyNetwork,
    app_networks: &HashMap<String, String>,
    running_apps: &HashSet<String>,
) -> NetworkVerdict {
    // Prefer the name-derived match: it is authoritative, since it is built from
    // the same function that creates the network. Fall back to the label for
    // networks whose app is no longer in the list.
    let app = app_networks.get(&network.name).cloned().or_else(|| {
        network
            .app
            .clone()
            .filter(|a| app_networks.values().any(|v| v == a))
    });

    match app {
        Some(app) if running_apps.contains(&app) => NetworkVerdict::Desired,
        Some(_) => NetworkVerdict::Ignore,
        None if network.managed => NetworkVerdict::OrphanCandidate,
        None => NetworkVerdict::Ignore,
    }
}

/// Whether an orphaned network can be removed.
///
/// True only when nothing but Traefik is attached. Any other endpoint means some
/// app's containers are still using this network — which is the guard that makes
/// pruning safe even if the app list is incomplete.
fn is_prunable(attached_containers: &[String], traefik_container: &str) -> bool {
    attached_containers
        .iter()
        .all(|name| name == traefik_container)
}

/// Whether the app currently needs to be reachable through the load balancer at
/// all: it declares public services *and* something of it is up.
///
/// This is the single gate in front of every other connectivity verdict. A
/// stopped app is unreachable by definition with nothing to repair, and an app
/// with no public services never wanted routing — flagging either would light up
/// the indicator for apps that are fine and make it meaningless for the case
/// that matters. Keeping the two conditions in one predicate is deliberate: when
/// they were checked separately around the load-balancer check, a stopped app
/// was reported as `LoadBalancerUnavailable` whenever the Traefik container
/// could not be inspected.
fn needs_routing(app: &AppData) -> bool {
    has_public_services(app) && is_running(app)
}

/// Derives the connectivity state to report for one app.
///
/// Uses the same predicate as the unroutable metric, so the reported state and
/// the metric can never disagree.
fn connectivity_for(
    app: &AppData,
    network: Option<&str>,
    attached: &HashSet<String>,
    load_balancer_available: bool,
) -> LoadBalancerConnectivity {
    if !needs_routing(app) {
        return LoadBalancerConnectivity::NotApplicable;
    }
    if !load_balancer_available {
        return LoadBalancerConnectivity::LoadBalancerUnavailable;
    }
    match network {
        // No per-app proxy network exists: either the app predates them and
        // routes over the shared base network, or it has not been deployed yet.
        // Either way this module has nothing to say about it.
        None => LoadBalancerConnectivity::NotApplicable,
        Some(network) if attached.contains(network) => LoadBalancerConnectivity::Connected,
        Some(_) => LoadBalancerConnectivity::Disconnected,
    }
}

/// Full reconciliation pass: repairs membership, prunes orphaned networks, and
/// annotates every app with the connectivity it observed.
///
/// Call this from the running-app check with its freshly discovered app list.
/// Being driven by a successful discovery is what makes pruning safe: a failed
/// discovery never reaches this function, so a transient I/O error on the apps
/// root can never be read as "all apps are gone".
#[instrument(skip(app_state, apps))]
pub async fn reconcile_traefik_networks(
    app_state: &SharedAppState,
    apps: &mut AppDataVec,
) -> anyhow::Result<()> {
    let _guard = PASS_LOCK.lock().await;
    run_pass(app_state, &mut apps.apps, true).await
}

/// Reconciliation pass driven by something other than the app check — currently
/// the Docker event watcher.
///
/// Works from the cached app list, writes the observed connectivity back, and
/// tells connected clients to refresh. It never prunes: the cache can be up to
/// one check interval stale, and while a stale list can only delay a *connect* to
/// the next scheduled pass, it could make a *removal* wrong.
///
/// The write-back deliberately patches one field per app via
/// [`SharedAppList::set_load_balancer_connectivity`] instead of storing the
/// mutated snapshot. This pass does slow Docker I/O between reading the cache and
/// writing to it, and the cache has other writers that know nothing about
/// [`PASS_LOCK`] — every state-machine transition, notification change and custom
/// action calls `update_app` directly. Writing whole `AppData` values back would
/// revert whatever they changed in that window.
#[instrument(skip(app_state))]
pub async fn reconcile_from_cache(app_state: &SharedAppState) -> anyhow::Result<()> {
    let Ok(_guard) = PASS_LOCK.try_lock() else {
        debug!("Skipping event-driven reconciliation, a pass is already running");
        return Ok(());
    };

    let mut apps = app_state.apps.get_apps().await.apps;
    if apps.is_empty() {
        debug!("No apps known yet, nothing to reconcile");
        return Ok(());
    }

    run_pass(app_state, &mut apps, false).await?;

    // Only apps whose connectivity actually changed are touched, so a converged
    // host does no cache writes and broadcasts nothing.
    let mut changed = 0;
    for app in &apps {
        if app_state
            .apps
            .set_load_balancer_connectivity(&app.name, app.load_balancer_connectivity)
            .await
        {
            changed += 1;
        }
    }

    if changed > 0 {
        debug!("Connectivity changed for {changed} app(s), notifying clients");
        app_state
            .messenger
            .broadcast_to_all(scotty_core::websocket::message::WebSocketMessage::AppListUpdated)
            .await;
    }

    Ok(())
}

/// The single implementation behind both passes.
///
/// `may_prune` is the only difference between them, so the connect and annotate
/// behaviour cannot diverge between triggers.
async fn run_pass(
    app_state: &SharedAppState,
    apps: &mut [AppData],
    may_prune: bool,
) -> anyhow::Result<()> {
    let Some(target) = traefik_target(&app_state.settings) else {
        // Not Traefik: no Docker calls, no drift, and connectivity does not
        // apply to any app.
        for app in apps.iter_mut() {
            app.load_balancer_connectivity = LoadBalancerConnectivity::NotApplicable;
        }
        return Ok(());
    };

    // 1. What is Traefik attached to right now?
    let attached = match traefik_networks(app_state, &target).await {
        Ok(attached) => attached,
        Err(e) => {
            // The load balancer itself is missing or unreadable, so nothing can
            // be routable. Report it once, mark the affected apps, and let the
            // check complete: this must never fail the surrounding task.
            error!(
                "Could not inspect Traefik container '{}', apps with public services cannot be made routable: {}",
                target.container, e
            );
            let mut outcome = PassOutcome::default();
            for app in apps.iter_mut() {
                app.load_balancer_connectivity =
                    connectivity_for(app, None, &HashSet::new(), false);
                if app.load_balancer_connectivity.is_problem() {
                    outcome.unroutable += 1;
                }
            }
            record(&outcome);
            return Ok(());
        }
    };

    // 2. Which proxy networks exist?
    let networks = match list_proxy_networks(app_state).await {
        Ok(networks) => networks,
        Err(e) => {
            // Deliberately no `record()` here: this pass learned nothing, and
            // reporting zero drift would be a false all-clear. Leaving the gauges
            // at their previous value is the honest option — see `record`.
            error!("Could not list Docker networks, skipping reconciliation pass: {e}");
            return Ok(());
        }
    };

    let app_networks: HashMap<String, String> = apps
        .iter()
        .map(|app| (target.network_for(&app.name), app.name.clone()))
        .collect();
    let running_apps: HashSet<String> = apps
        .iter()
        .filter(|app| is_running(app))
        .map(|app| app.name.clone())
        .collect();
    let by_name: HashMap<&str, &ProxyNetwork> =
        networks.iter().map(|n| (n.name.as_str(), n)).collect();

    // 3. Connect what should be connected, then record what we see.
    let mut attached = attached;
    let mut outcome = PassOutcome::default();

    for app in apps.iter_mut() {
        let network = target.network_for(&app.name);
        // Both loops in this pass ask `classify` rather than re-deriving the
        // decision, so "which networks does Traefik belong on" has exactly one
        // implementation — and it is the one the unit tests cover.
        let verdict = by_name
            .get(network.as_str())
            .map(|n| classify(n, &app_networks, &running_apps));
        let network_exists = verdict.is_some();

        if verdict == Some(NetworkVerdict::Desired) && !attached.contains(&network) {
            outcome.drifted += 1;
            warn!(
                "App '{}' is not reachable by Traefik: '{}' is not attached to network {}. Reconnecting.",
                app.name, target.container, network
            );
            match connect(app_state, &target, &network).await {
                Ok(()) => {
                    info!(
                        "Reconnected Traefik ('{}') to network {} for app '{}'",
                        target.container, network, app.name
                    );
                    attached.insert(network.clone());
                }
                Err(e) => {
                    // Per-app failure: report it and keep going, so one broken
                    // app cannot hide the state of the others.
                    error!(
                        "Failed to reconnect Traefik ('{}') to network {} for app '{}': {}",
                        target.container, network, app.name, e
                    );
                }
            }
        }

        app.load_balancer_connectivity = connectivity_for(
            app,
            network_exists.then_some(network.as_str()),
            &attached,
            true,
        );
        if app.load_balancer_connectivity.is_problem() {
            outcome.unroutable += 1;
        }
    }

    // 4. Clean up networks whose app is gone.
    if may_prune {
        prune_orphans(app_state, &target, &networks, &app_networks, &running_apps).await;
    }

    record(&outcome);
    if outcome == PassOutcome::default() {
        debug!(
            "Traefik proxy networks converged ({} apps checked)",
            apps.len()
        );
    }

    Ok(())
}

/// Names of the networks the Traefik container is currently attached to.
async fn traefik_networks(
    app_state: &SharedAppState,
    target: &TraefikTarget,
) -> anyhow::Result<HashSet<String>> {
    let insights = app_state
        .docker
        .inspect_container(&target.container, None::<InspectContainerOptions>)
        .await?;

    Ok(insights
        .network_settings
        .and_then(|s| s.networks)
        .map(|networks| networks.into_keys().collect())
        .unwrap_or_default())
}

/// All networks on the host that could be per-app proxy networks.
///
/// Listed unfiltered rather than by `scotty.managed=true`, because the classifier
/// also has to recognise per-app networks that are missing the label (created by
/// an earlier version) and to know whether an app's network exists at all. The
/// label is still what gates pruning; it is read per network here.
async fn list_proxy_networks(app_state: &SharedAppState) -> anyhow::Result<Vec<ProxyNetwork>> {
    let networks = app_state
        .docker
        .list_networks(None::<ListNetworksOptions>)
        .await?;

    Ok(networks
        .into_iter()
        .filter_map(|network| {
            let name = network.name?;
            let labels = network.labels.unwrap_or_default();
            Some(ProxyNetwork {
                name,
                app: labels.get(LABEL_APP).cloned(),
                managed: labels.get(LABEL_MANAGED).map(String::as_str) == Some("true"),
            })
        })
        .collect())
}

/// Attaches Traefik to a network, tolerating the benign outcomes.
///
/// 403/409 mean "already connected" (which status a daemon returns is
/// version-dependent) and 404 means the network or the container disappeared
/// between listing and connecting, e.g. because a destroy ran concurrently.
/// Neither is an error worth reporting.
async fn connect(
    app_state: &SharedAppState,
    target: &TraefikTarget,
    network: &str,
) -> anyhow::Result<()> {
    match app_state
        .docker
        .connect_network(
            network,
            NetworkConnectRequest {
                container: target.container.clone(),
                endpoint_config: None,
            },
        )
        .await
    {
        Ok(()) => Ok(()),
        Err(e) if matches!(server_status(&e), Some(403 | 409)) => Ok(()),
        Err(e) if server_status(&e) == Some(404) => {
            debug!(
                "Network {} or container '{}' vanished while reconnecting",
                network, target.container
            );
            Ok(())
        }
        Err(e) => Err(anyhow::Error::from(e)),
    }
}

/// Removes proxy networks whose app no longer exists.
///
/// Best-effort throughout: every failure is logged and skipped, because a
/// network left behind is a cosmetic problem while a failed pass is not.
async fn prune_orphans(
    app_state: &SharedAppState,
    target: &TraefikTarget,
    networks: &[ProxyNetwork],
    app_networks: &HashMap<String, String>,
    running_apps: &HashSet<String>,
) {
    for network in networks {
        if classify(network, app_networks, running_apps) != NetworkVerdict::OrphanCandidate {
            continue;
        }

        let inspected = match app_state
            .docker
            .inspect_network(&network.name, None::<InspectNetworkOptions>)
            .await
        {
            Ok(inspected) => inspected,
            Err(e) if server_status(&e) == Some(404) => continue,
            Err(e) => {
                info!("Skipping orphaned network {}: {}", network.name, e);
                continue;
            }
        };

        let attached: Vec<String> = inspected
            .containers
            .unwrap_or_default()
            .into_values()
            .filter_map(|endpoint| endpoint.name)
            .collect();

        if !is_prunable(&attached, &target.container) {
            info!(
                "Skipping orphaned network {}: still has non-Traefik endpoints attached ({})",
                network.name,
                attached.join(", ")
            );
            continue;
        }

        // Disconnect first: Docker refuses to remove a network while an endpoint
        // is attached. `force` is safe here because the app is gone, so there is
        // no in-flight request to disrupt.
        if let Err(e) = app_state
            .docker
            .disconnect_network(
                &network.name,
                NetworkDisconnectRequest {
                    container: target.container.clone(),
                    force: Some(true),
                },
            )
            .await
        {
            if !matches!(server_status(&e), Some(403 | 404 | 409)) {
                info!(
                    "Could not disconnect Traefik from orphaned network {}: {}",
                    network.name, e
                );
                continue;
            }
        }

        match app_state.docker.remove_network(&network.name).await {
            Ok(()) => info!(
                "Removed orphaned proxy network {} (app '{}' no longer exists)",
                network.name,
                network.app.as_deref().unwrap_or("unknown")
            ),
            Err(e) if server_status(&e) == Some(404) => {}
            Err(e) => info!(
                "Could not remove orphaned proxy network {}: {}",
                network.name, e
            ),
        }
    }
}

/// Publishes the outcome of a pass, including zeroes, so a healthy host reports
/// "0 unroutable" rather than no data at all.
///
/// One exception: a pass that could not list the host's networks does not call
/// this. It has no idea what the state is, and a gauge cannot say "unknown" —
/// reporting zero would look like an all-clear, so the previous values are left
/// standing instead.
fn record(outcome: &PassOutcome) {
    let m = crate::metrics::metrics();
    m.record_traefik_network_drift_apps(outcome.drifted as u64);
    m.record_traefik_network_unroutable_apps(outcome.unroutable as u64);
}

/// Watches Docker for the Traefik container starting, and reconciles when it
/// does.
///
/// This is strictly an accelerator on top of the scheduled pass: it shortens the
/// repair window from up to one `running_app_check` interval to seconds, and
/// every repair it makes would happen anyway on the next scheduled pass. It is
/// therefore free to fail — failures are logged and retried, never propagated.
#[instrument(skip(app_state))]
pub async fn watch_traefik_events(app_state: SharedAppState) {
    let Some(target) = traefik_target(&app_state.settings) else {
        return;
    };

    let mut filters = HashMap::new();
    filters.insert("type".to_string(), vec!["container".to_string()]);
    filters.insert("event".to_string(), vec!["start".to_string()]);
    filters.insert("container".to_string(), vec![target.container.clone()]);
    let options = EventsOptions {
        since: None,
        until: None,
        filters: Some(filters),
    };

    let mut backoff = Duration::from_secs(1);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);

    info!(
        "Watching Docker events for container '{}' starts",
        target.container
    );

    while !app_state.stop_flag.is_stopped() {
        // Reconcile on every (re)subscribe, not only on events: a container
        // start that happened while the stream was down would otherwise wait
        // for the next scheduled pass.
        if let Err(e) = reconcile_from_cache(&app_state).await {
            error!("Reconciliation on event-stream (re)subscribe failed: {e:?}");
        }

        let mut stream = app_state.docker.events(Some(options.clone()));
        let mut stream_ok = true;

        while let Some(event) = stream.next().await {
            if app_state.stop_flag.is_stopped() {
                return;
            }
            match event {
                Ok(event) => {
                    if !is_container_start(&event) {
                        continue;
                    }
                    info!(
                        "Container '{}' started, reconciling proxy networks",
                        target.container
                    );
                    // Coalesce bursts: recreating a container emits `die` then
                    // `start`, and compose can emit several in a row. Waiting
                    // briefly lets the burst settle into one pass, and the
                    // single-flight lock drops any that still overlap.
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    if let Err(e) = reconcile_from_cache(&app_state).await {
                        error!("Reconciliation after Traefik start failed: {e:?}");
                    }
                    // A successful event resets the backoff: the stream is
                    // healthy regardless of how it failed before.
                    backoff = Duration::from_secs(1);
                }
                Err(e) => {
                    warn!("Docker event stream failed, will resubscribe: {e}");
                    stream_ok = false;
                    break;
                }
            }
        }

        if app_state.stop_flag.is_stopped() {
            return;
        }
        if stream_ok {
            debug!("Docker event stream ended, resubscribing");
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// Whether an event message is a container start.
///
/// The daemon filters already narrow this down, but the filters are advisory on
/// old daemons and cheap to re-check.
fn is_container_start(event: &bollard_stubs::models::EventMessage) -> bool {
    event.typ == Some(EventMessageTypeEnum::CONTAINER) && event.action.as_deref() == Some("start")
}

#[cfg(test)]
mod tests {
    use super::*;
    use scotty_core::apps::app_data::{
        AppSettings, ContainerState, ContainerStatus, ServicePortMapping,
    };

    fn container(status: ContainerStatus) -> ContainerState {
        ContainerState {
            status,
            id: Some("cafe".to_string()),
            service: "web".to_string(),
            domains: vec![],
            use_tls: false,
            port: None,
            started_at: None,
            used_registry: None,
            basic_auth: None,
            exit_code: None,
        }
    }

    fn app(name: &str, status: ContainerStatus, public: bool) -> AppData {
        let settings = AppSettings {
            public_services: if public {
                vec![ServicePortMapping {
                    service: "web".to_string(),
                    port: 80,
                    domains: vec![format!("{name}.example.com")],
                }]
            } else {
                vec![]
            },
            ..Default::default()
        };
        AppData {
            name: name.to_string(),
            services: vec![container(status)],
            settings: Some(settings),
            ..Default::default()
        }
    }

    fn target() -> TraefikTarget {
        TraefikTarget {
            base_network: "proxy".to_string(),
            container: "traefik".to_string(),
        }
    }

    /// Builds the lookups `classify` needs from a list of apps, the same way
    /// `run_pass` does.
    fn lookups(apps: &[AppData]) -> (HashMap<String, String>, HashSet<String>) {
        let t = target();
        (
            apps.iter()
                .map(|a| (t.network_for(&a.name), a.name.clone()))
                .collect(),
            apps.iter()
                .filter(|a| is_running(a))
                .map(|a| a.name.clone())
                .collect(),
        )
    }

    fn network(name: &str, app: Option<&str>, managed: bool) -> ProxyNetwork {
        ProxyNetwork {
            name: name.to_string(),
            app: app.map(String::from),
            managed,
        }
    }

    #[test]
    fn running_app_network_is_desired() {
        let apps = vec![app("blog", ContainerStatus::Running, true)];
        let (nets, running) = lookups(&apps);
        assert_eq!(
            classify(&network("proxy--blog", Some("blog"), true), &nets, &running),
            NetworkVerdict::Desired
        );
    }

    #[test]
    fn stopped_app_network_is_ignored() {
        let apps = vec![app("blog", ContainerStatus::Exited, true)];
        let (nets, running) = lookups(&apps);
        assert_eq!(
            classify(&network("proxy--blog", Some("blog"), true), &nets, &running),
            NetworkVerdict::Ignore
        );
    }

    #[test]
    fn unlabelled_network_of_running_app_is_still_desired() {
        // Created by a version that did not label networks: recognised by name,
        // so it is connected, but it can never be pruned.
        let apps = vec![app("blog", ContainerStatus::Running, true)];
        let (nets, running) = lookups(&apps);
        assert_eq!(
            classify(&network("proxy--blog", None, false), &nets, &running),
            NetworkVerdict::Desired
        );
    }

    #[test]
    fn managed_network_without_app_is_an_orphan_candidate() {
        let apps = vec![app("blog", ContainerStatus::Running, true)];
        let (nets, running) = lookups(&apps);
        assert_eq!(
            classify(&network("proxy--gone", Some("gone"), true), &nets, &running),
            NetworkVerdict::OrphanCandidate
        );
    }

    #[test]
    fn unmanaged_networks_are_never_orphan_candidates() {
        let apps = vec![app("blog", ContainerStatus::Running, true)];
        let (nets, running) = lookups(&apps);
        // The base network itself, and anything else on the host.
        assert_eq!(
            classify(&network("proxy", None, false), &nets, &running),
            NetworkVerdict::Ignore
        );
        assert_eq!(
            classify(&network("bridge", None, false), &nets, &running),
            NetworkVerdict::Ignore
        );
        assert_eq!(
            classify(
                &network("some-other-stack_default", None, false),
                &nets,
                &running
            ),
            NetworkVerdict::Ignore
        );
    }

    #[test]
    fn prunable_only_without_foreign_endpoints() {
        assert!(is_prunable(&[], "traefik"));
        assert!(is_prunable(&["traefik".to_string()], "traefik"));
        assert!(!is_prunable(
            &["traefik".to_string(), "gone-web-1".to_string()],
            "traefik"
        ));
        assert!(!is_prunable(&["gone-web-1".to_string()], "traefik"));
    }

    #[test]
    fn connectivity_reports_connected_and_disconnected() {
        let app = app("blog", ContainerStatus::Running, true);
        let attached: HashSet<String> = ["proxy--blog".to_string()].into_iter().collect();
        assert_eq!(
            connectivity_for(&app, Some("proxy--blog"), &attached, true),
            LoadBalancerConnectivity::Connected
        );
        assert_eq!(
            connectivity_for(&app, Some("proxy--blog"), &HashSet::new(), true),
            LoadBalancerConnectivity::Disconnected
        );
    }

    #[test]
    fn connectivity_is_not_applicable_without_public_services() {
        let app = app("worker", ContainerStatus::Running, false);
        assert_eq!(
            connectivity_for(&app, Some("proxy--worker"), &HashSet::new(), true),
            LoadBalancerConnectivity::NotApplicable
        );
        // Not even when the load balancer is down: nothing declares that this
        // app should be reachable.
        assert_eq!(
            connectivity_for(&app, None, &HashSet::new(), false),
            LoadBalancerConnectivity::NotApplicable
        );
    }

    #[test]
    fn connectivity_is_not_applicable_for_legacy_and_stopped_apps() {
        // Legacy app: routes over the shared base network, has no per-app one.
        let legacy = app("legacy", ContainerStatus::Running, true);
        assert_eq!(
            connectivity_for(&legacy, None, &HashSet::new(), true),
            LoadBalancerConnectivity::NotApplicable
        );
        // Stopped app: nothing to repair, and flagging it would make the
        // indicator meaningless.
        let stopped = app("blog", ContainerStatus::Exited, true);
        assert_eq!(
            connectivity_for(&stopped, Some("proxy--blog"), &HashSet::new(), true),
            LoadBalancerConnectivity::NotApplicable
        );
    }

    #[test]
    fn connectivity_reports_missing_load_balancer() {
        let app = app("blog", ContainerStatus::Running, true);
        assert_eq!(
            connectivity_for(&app, None, &HashSet::new(), false),
            LoadBalancerConnectivity::LoadBalancerUnavailable
        );
    }

    /// A failure to inspect the Traefik container marks every app at once, so the
    /// "does this app need routing" gate has to come first: otherwise a daemon
    /// hiccup or a mid-recreate Traefik reports every *stopped* app with public
    /// services as unroutable and inflates the metric exactly when an operator is
    /// using it to judge blast radius.
    #[test]
    fn stopped_apps_are_not_unroutable_when_the_load_balancer_is_missing() {
        let stopped = app("blog", ContainerStatus::Exited, true);
        assert_eq!(
            connectivity_for(&stopped, Some("proxy--blog"), &HashSet::new(), false),
            LoadBalancerConnectivity::NotApplicable
        );
        assert_eq!(
            connectivity_for(&stopped, None, &HashSet::new(), false),
            LoadBalancerConnectivity::NotApplicable
        );

        // An app with no public services is equally unaffected, running or not.
        let private = app("worker", ContainerStatus::Running, false);
        assert_eq!(
            connectivity_for(&private, None, &HashSet::new(), false),
            LoadBalancerConnectivity::NotApplicable
        );

        // Only a running app that wants routing is reported as affected.
        let running = app("blog", ContainerStatus::Running, true);
        assert!(
            connectivity_for(&running, None, &HashSet::new(), false).is_problem(),
            "a running public app must still be counted when the LB is gone"
        );
    }

    #[test]
    fn needs_routing_requires_both_public_services_and_a_running_container() {
        assert!(needs_routing(&app("blog", ContainerStatus::Running, true)));
        assert!(!needs_routing(&app("blog", ContainerStatus::Exited, true)));
        assert!(!needs_routing(&app(
            "worker",
            ContainerStatus::Running,
            false
        )));
        assert!(!needs_routing(&app(
            "worker",
            ContainerStatus::Exited,
            false
        )));
    }

    #[test]
    fn only_definite_failures_count_as_problems() {
        assert!(LoadBalancerConnectivity::Disconnected.is_problem());
        assert!(LoadBalancerConnectivity::LoadBalancerUnavailable.is_problem());
        assert!(!LoadBalancerConnectivity::Connected.is_problem());
        assert!(!LoadBalancerConnectivity::NotApplicable.is_problem());
        assert!(!LoadBalancerConnectivity::Unknown.is_problem());
    }

    #[test]
    fn created_and_restarting_apps_count_as_running() {
        assert!(is_running(&app("a", ContainerStatus::Running, true)));
        assert!(is_running(&app("a", ContainerStatus::Created, true)));
        assert!(is_running(&app("a", ContainerStatus::Restarting, true)));
        assert!(!is_running(&app("a", ContainerStatus::Exited, true)));
        assert!(!is_running(&app("a", ContainerStatus::Empty, true)));
    }
}
