## Purpose

Defines how Scotty keeps the load balancer's membership in per-app proxy networks converged with the set of deployed apps, so that recreating or replacing the Traefik container never leaves already-running apps silently unroutable, and so that stale proxy networks do not accumulate.

## ADDED Requirements

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

### Requirement: Reconciliation never disrupts working routing

Reconciliation SHALL be safe to run repeatedly and concurrently with app deployments. Attaching an already-attached load balancer, removing an already-removed network, and a load balancer container that is absent SHALL all be tolerated without failing the running-app check. A failure to reconcile one app SHALL NOT prevent the remaining apps from being reconciled, and no reconciliation outcome SHALL stop, restart, or otherwise alter app containers.

#### Scenario: Load balancer container is missing

- **WHEN** reconciliation cannot find the configured load balancer container
- **THEN** the running-app check completes successfully
- **AND** the system reports that apps with public services cannot be made routable

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
