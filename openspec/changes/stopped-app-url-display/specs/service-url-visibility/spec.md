# service-url-visibility

## ADDED Requirements

### Requirement: Service URLs are shown regardless of container status
The frontend SHALL render a service's domain URLs as clickable links whenever the service has one or more domains, regardless of the service's container status. The link target, label, icon, and layout SHALL be identical for running and non-running services.

#### Scenario: Running service with domains
- **WHEN** a service has status `Running` and at least one domain
- **THEN** each domain is rendered as a clickable link in the normal (running) style

#### Scenario: Stopped service with domains
- **WHEN** a service has a status other than `Running` (e.g. `Exited`, `Empty`, `Created`) and at least one domain
- **THEN** each domain is rendered as a clickable link with the same layout and target as for a running service

### Requirement: Non-running service URLs are visually distinguished
URL links for services that are not `Running` SHALL use a distinct muted/dimmed color treatment so users can tell at a glance that the service is not up. Running services SHALL keep the current styling.

#### Scenario: Stopped service link styling
- **WHEN** a service that is not `Running` renders its domain links
- **THEN** the links use the muted non-running style, visually distinct from running-service links

### Requirement: Services without domains keep the fallback button
A service with no domains SHALL keep the current fallback rendering: a non-clickable button showing the service name.

#### Scenario: Service without domains
- **WHEN** a service has no domains (any status)
- **THEN** a disabled button with the service name is shown and no link is rendered
