# traefik-network-reconciliation Specification

## Purpose

Defines how Scotty keeps the load balancer's membership in per-app proxy networks converged with the set of deployed apps, so that recreating or replacing the Traefik container never leaves already-running apps silently unroutable, and so that stale proxy networks do not accumulate. Also defines how each app's resulting load-balancer connectivity is reported through logs, metrics, the app detail data, and the web UI, so the condition is visible instead of silent.

## Requirements

### Requirement: Proxy network membership is reconciled, not only set at deploy time

When the configured load balancer is Traefik, the system SHALL periodically reconcile the load balancer's membership in per-app proxy networks against the set of deployed apps, in addition to establishing that membership when an app is deployed. Reconciliation SHALL run as part of the running-app check — both the check performed at startup and every scheduled repetition of it — so that a load balancer container recreated at any time is repaired without operator intervention and without redeploying apps.

#### Scenario: Load balancer container recreated while apps are running

- **WHEN** the Traefik container is recreated so that it is attached only to the base proxy network, while apps with public services are still running
- **THEN** the next running-app check connects Traefik to each running app's per-app proxy network
- **AND** those apps become reachable again without any app being redeployed

#### Scenario: Membership already correct

- **WHEN** a reconciliation pass runs and Traefik is already attached to every per-app proxy network it should be attached to
- **THEN** the system makes no network attachment or removal changes
- **AND** reports no drift

#### Scenario: Reconciliation runs at startup

- **WHEN** the server starts up and performs its initial running-app check
- **THEN** proxy network membership is reconciled as part of that check, before any user request depends on it

### Requirement: Reconciliation is triggered by load balancer container lifecycle events

The system SHALL watch Docker container lifecycle events for the configured load balancer container and SHALL reconcile proxy network membership when that container starts, so that a recreated or restarted load balancer is repaired within seconds instead of after up to a full running-app check interval. The system SHALL also re-evaluate connectivity when that container stops, so that the reported state degrades as promptly as it recovers rather than remaining stale for up to a full running-app check interval. Event-triggered reconciliation SHALL apply the same connect, prune, and reporting rules as the periodic pass. Event watching SHALL be controlled by a configuration setting that defaults to enabled, and SHALL be independent of the periodic pass: disabling it SHALL NOT disable periodic reconciliation, and its failure SHALL NOT disable periodic reconciliation.

#### Scenario: Traefik container is recreated

- **WHEN** the Traefik container starts, having lost its per-app network attachments
- **THEN** the system reconciles proxy network membership without waiting for the next scheduled app check
- **AND** running apps become reachable again

#### Scenario: Traefik container is stopped

- **WHEN** the Traefik container stops while apps with public services are running
- **THEN** the system re-evaluates connectivity without waiting for the next scheduled app check
- **AND** those apps report the load-balancer-unavailable state within seconds rather than continuing to report the connected state

#### Scenario: Repeated events are coalesced

- **WHEN** several container lifecycle events for the load balancer arrive in quick succession
- **THEN** the system performs reconciliation without running overlapping passes
- **AND** does not issue redundant network changes

#### Scenario: Event watching disabled by configuration

- **WHEN** the event-watching setting is disabled
- **THEN** the system does not watch Docker events
- **AND** reconciliation still runs on startup and on every scheduled running-app check

#### Scenario: Event stream is interrupted

- **WHEN** the Docker event stream ends or fails
- **THEN** the system keeps retrying in the background without terminating the server or the scheduled checks
- **AND** reconciles when the stream is re-established, so any container start missed while disconnected is still repaired

#### Scenario: Events for other containers are ignored

- **WHEN** a container other than the configured load balancer starts
- **THEN** no event-triggered reconciliation is performed

#### Scenario: Event watching is inactive for non-Traefik load balancers

- **WHEN** the configured load balancer type is not Traefik
- **THEN** the system does not watch Docker events regardless of the setting value

### Requirement: Only existing per-app proxy networks of running apps are attached

Reconciliation SHALL connect the load balancer to a per-app proxy network only when that network already exists and belongs to a discovered app that has at least one running container. Reconciliation SHALL NOT create proxy networks, and SHALL NOT attach the load balancer to the proxy network of an app that has no running containers; creating and attaching networks for an app remains the responsibility of the app deployment flow.

#### Scenario: Stopped app is not attached

- **WHEN** a reconciliation pass encounters an existing per-app proxy network whose app is deployed but has no running containers
- **THEN** the system leaves the load balancer's membership for that network unchanged
- **AND** does not report drift for that app

#### Scenario: No proxy network exists for an app

- **WHEN** a discovered app has running containers but no per-app proxy network exists for it
- **THEN** the system does not create the network during reconciliation
- **AND** the app is left for its normal deployment flow to set up

### Requirement: Orphaned proxy networks are pruned safely

Reconciliation SHALL remove per-app proxy networks that Scotty created for apps that no longer exist, so that load balancer membership and the Docker network list do not grow without bound. A network SHALL be treated as prunable only when all of the following hold: it is marked as Scotty-managed, the app it names is not among the discovered apps, and no container other than the load balancer is attached to it. Any network that does not meet all three conditions SHALL be left untouched.

#### Scenario: Proxy network of a removed app is cleaned up

- **WHEN** a Scotty-managed per-app proxy network names an app that is no longer deployed, and only the load balancer is attached to it
- **THEN** the system disconnects the load balancer from that network and removes the network

#### Scenario: Network with foreign containers attached is preserved

- **WHEN** a Scotty-managed per-app proxy network names an app that is not among the discovered apps, but a container other than the load balancer is still attached to it
- **THEN** the system leaves the network and the load balancer's membership in it unchanged
- **AND** records that the network was skipped

#### Scenario: Unmanaged network is never removed

- **WHEN** a network exists that is not marked as Scotty-managed, including the base proxy network itself
- **THEN** reconciliation never disconnects the load balancer from it and never removes it

#### Scenario: App discovery failed

- **WHEN** the app list could not be determined for the current check
- **THEN** reconciliation performs no pruning in that pass

### Requirement: Load balancer availability means running, not merely present

The system SHALL treat the load balancer as available only when its container both exists **and** is running. A container that exists but is not running SHALL be treated exactly as an absent one, and the network attachments recorded on it SHALL NOT be read as evidence that any app is reachable: Docker retains a container's network attachments while it is stopped, so those attachments outlive the container's ability to route traffic. Whenever the load balancer is unavailable, every app that needs routing SHALL report the load-balancer-unavailable state and SHALL NOT report the connected state.

#### Scenario: Stopped load balancer still lists per-app networks

- **WHEN** the load balancer container is stopped while still recorded as attached to a running app's per-app proxy network
- **THEN** the app reports the load-balancer-unavailable state
- **AND** it does not report the connected state on the basis of that retained attachment

#### Scenario: Running load balancer attached to the app's network

- **WHEN** the load balancer container is running and attached to a running app's per-app proxy network
- **THEN** the app reports the connected state

#### Scenario: Availability is judged the same way on every trigger

- **WHEN** availability is evaluated during a scheduled pass, a startup pass, or an event-triggered pass
- **THEN** the same running-container rule applies, so the reported state cannot differ by which trigger observed it

### Requirement: Reconciliation never disrupts working routing

Reconciliation SHALL be safe to run repeatedly and concurrently with app deployments. Attaching an already-attached load balancer, removing an already-removed network, and a load balancer container that is absent or not running SHALL all be tolerated without failing the running-app check. A failure to reconcile one app SHALL NOT prevent the remaining apps from being reconciled, and no reconciliation outcome SHALL stop, restart, or otherwise alter app containers.

#### Scenario: Load balancer container is missing

- **WHEN** reconciliation cannot find the configured load balancer container
- **THEN** the running-app check completes successfully
- **AND** the system reports that apps with public services cannot be made routable

#### Scenario: Load balancer container exists but is not running

- **WHEN** the configured load balancer container exists but is stopped, exited, created, or otherwise not running
- **THEN** the running-app check completes successfully
- **AND** the system reports that apps with public services cannot be made routable, exactly as when the container is absent

#### Scenario: One app fails to reconcile

- **WHEN** connecting the load balancer to one app's proxy network fails
- **THEN** the system still reconciles the other apps in the same pass
- **AND** reports the failure for the affected app

#### Scenario: Deployment in flight

- **WHEN** an app is being deployed at the same time as a reconciliation pass, so its proxy network may appear or disappear mid-pass
- **THEN** reconciliation tolerates the missing network or already-present attachment without error
- **AND** the app's deployment outcome is unaffected

### Requirement: Non-Traefik load balancers are unaffected

When the configured load balancer type is not Traefik, reconciliation SHALL do nothing: it SHALL make no Docker network calls and report no drift.

#### Scenario: HAProxy configuration in use

- **WHEN** the load balancer type is the legacy HAProxy configuration
- **THEN** reconciliation is a no-op

### Requirement: Apps on the legacy shared proxy network are left alone

Apps whose services still attach to the single shared base proxy network instead of a per-app proxy network SHALL be unaffected by reconciliation. The system SHALL NOT create a per-app network for them, SHALL NOT change their routing, and SHALL NOT report them as drifted.

#### Scenario: Legacy app not yet migrated

- **WHEN** a running app predates per-app proxy networks and routes over the shared base network
- **THEN** reconciliation leaves it untouched and reports no drift for it

### Requirement: Routing drift is reported to operators

The system SHALL make the outage condition observable rather than silent. When reconciliation finds that a running app with public services is not reachable by the load balancer, it SHALL log a warning identifying the app and the network before repairing it, and SHALL log an error when the condition cannot be repaired. The system SHALL also expose the number of apps found drifted and the number of apps still unroutable after a pass as metrics, so the condition can be alerted on.

#### Scenario: Drift detected and repaired

- **WHEN** reconciliation finds a running app with public services whose proxy network the load balancer is not attached to
- **THEN** a warning is logged naming the app and the network
- **AND** the repair is logged
- **AND** the pass reports at least one drifted app in its metrics

#### Scenario: Drift cannot be repaired

- **WHEN** reconciliation cannot attach the load balancer to a running app's proxy network
- **THEN** an error is logged naming the app, the network, and the reason
- **AND** the pass reports at least one still-unroutable app in its metrics

#### Scenario: Healthy pass is quiet

- **WHEN** a reconciliation pass finds no drift
- **THEN** no warning or error is logged for routing drift
- **AND** the drifted and unroutable counts reported are zero

### Requirement: App detail data reports load-balancer connectivity

The app detail data returned for an app SHALL include that app's load-balancer connectivity as an explicit state, distinguishing at least: the load balancer is running and attached to the app's proxy network; not reachable because the load balancer is not attached to the app's proxy network; not reachable because the load balancer itself is unavailable; not applicable (no public services, a non-Traefik load balancer, or an app that does not use a per-app proxy network); and not yet determined. The state SHALL reflect what was observed from Docker during the most recent reconciliation, not an assumption derived from the app's status. Clients that do not send or understand the field SHALL remain compatible: the field SHALL be optional on the wire in both directions.

**Scope limitation — the load-balancer side only.** The connected state asserts one fact: the load balancer is running and joined to the app's proxy network. It does NOT assert that requests to the app's domains succeed. In particular it does not verify that the app's own containers are joined to that same network, that the app declares usable Traefik labels, that referenced middlewares exist, or that the app answers on its configured port. An app can therefore report the connected state and still return an HTTP error. This is deliberate: the state's job is to make the one failure mode that reconciliation owns — a load balancer that lost its network membership — visible and machine-readable, not to be a health check. Consumers SHALL NOT present the connected state as a guarantee of reachability.

#### Scenario: Only the load balancer is attached to the proxy network

- **WHEN** the load balancer is running and attached to a running app's per-app proxy network, but the app's own containers are not attached to that network (for example an app whose override still joins the legacy shared network)
- **THEN** the app reports the connected state, because the load-balancer side of the network is in the desired condition
- **AND** requests to the app's domains may still fail, which this state does not claim to detect

#### Scenario: Connected app

- **WHEN** a client requests the detail data for a running app with public services whose proxy network the load balancer is attached to
- **THEN** the connectivity state reports that the app is reachable by the load balancer

#### Scenario: Unroutable app

- **WHEN** a running app with public services is not reachable because the load balancer could not be attached to its proxy network
- **THEN** the connectivity state reports it as not reachable
- **AND** the app's own status is unchanged, since its containers are running

#### Scenario: Load balancer unavailable

- **WHEN** the configured load balancer container does not exist, or exists but is not running
- **THEN** apps with public services report the load-balancer-unavailable state rather than a plain connected or disconnected state

#### Scenario: Not applicable

- **WHEN** an app has no public services, or the load balancer is not Traefik, or the app routes over the legacy shared network
- **THEN** the connectivity state reports that connectivity is not applicable

#### Scenario: Before the first reconciliation

- **WHEN** app detail data is produced by a path that has not yet observed connectivity, such as immediately after a deploy and before the next reconciliation pass
- **THEN** the connectivity state reports that it is not yet determined, rather than guessing connected or disconnected

#### Scenario: Older client reads the new payload

- **WHEN** a client built before this change deserializes app detail data containing the connectivity field
- **THEN** it ignores the field and continues to work

#### Scenario: Newer client reads an older payload

- **WHEN** a client built after this change deserializes app detail data from a server that does not send the connectivity field
- **THEN** the state is treated as not yet determined rather than failing to parse

### Requirement: Web UI shows a connectivity indicator on the app detail page

The web UI SHALL display an indicator of the app's load-balancer connectivity on the app detail page, alongside the app status, so that an app whose containers are running but whose load balancer has lost its proxy network membership is visually distinguishable from a healthy one. The indicator SHALL visually distinguish the connected state from the unreachable states, SHALL convey which unreachable condition applies, and SHALL NOT add visual noise for apps where connectivity is not applicable or not yet determined.

Because the connected state covers the load-balancer side of the network only (see the scope limitation above), the indicator's explanatory text for that state SHALL describe what was observed — that the load balancer is attached to the app's proxy network — rather than promise that requests succeed, so a connected app that nonetheless errors does not make the indicator read as wrong.

#### Scenario: Connected state describes what was observed

- **WHEN** the indicator shows the connected state
- **THEN** its explanatory text states that the load balancer is attached to the app's proxy network
- **AND** it does not claim that the app's domains serve requests successfully

#### Scenario: Unroutable app is visible in the UI

- **WHEN** a user opens the detail page of a running app that the load balancer cannot reach
- **THEN** the page shows an indicator marking the app as not reachable, distinct from the running status shown for its containers

#### Scenario: Healthy app in the UI

- **WHEN** a user opens the detail page of a running app that is reachable by the load balancer
- **THEN** the page shows the indicator in its reachable state

#### Scenario: Connectivity not applicable

- **WHEN** a user opens the detail page of an app with no public services
- **THEN** no connectivity indicator is shown

#### Scenario: Indicator follows live updates

- **WHEN** the app list is refreshed while the detail page is open and the app's connectivity has changed
- **THEN** the indicator updates without requiring a manual page reload
