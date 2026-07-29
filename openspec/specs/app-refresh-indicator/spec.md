# app-refresh-indicator Specification

## Purpose

Define what the app detail page tells the user about Scotty's app-state refresh cycle: how the time of the next scheduled app-state sweep is derived on the server, how it is exposed on app data, and how relative times are rendered in both directions.

## Requirements

### Requirement: Server exposes the next scheduled app check

The server SHALL track the time of the next scheduled app-state sweep and expose it on app data as `next_check`. The value SHALL be derived from the scheduler's own cadence (`scheduler.running_app_check`), not extrapolated from the time an individual app was last inspected.

#### Scenario: Sweep records the next due time

- **WHEN** the scheduled app check begins
- **THEN** the next-due time is recorded as the sweep's start time plus the configured `running_app_check` interval
- **AND** app data inspected during that sweep reports that time as `next_check`

#### Scenario: Manual action does not shift the reported next check

- **WHEN** an app is run, stopped, or rebuilt between two sweeps, causing it to be inspected out of band
- **THEN** the app's `next_check` still reports the next scheduled sweep
- **AND** it is not moved to one interval after the manual inspection

#### Scenario: Never-swept app

- **WHEN** an app has been created but no app check has run yet
- **THEN** `next_check` is null

### Requirement: App detail page shows the next update as a relative time

The app detail page SHALL display the time until the next app-state refresh as a relative time, replacing the previous "last updated" indicator.

#### Scenario: Next check in the future

- **WHEN** the app's `next_check` is in the future
- **THEN** the pill reads that the next update is due in the remaining relative time (e.g. "Next update in 12 minutes")

#### Scenario: No next check known

- **WHEN** the app's `next_check` is null
- **THEN** no next-update pill is rendered

### Requirement: Relative-time rendering is sign-aware

The shared relative-time component SHALL render future timestamps as a time-until phrase and past timestamps as a time-ago phrase, using the same unit selection for both directions.

#### Scenario: Future timestamp

- **WHEN** the component is given a timestamp later than the current time
- **THEN** it renders a forward-looking phrase (e.g. "in 12 minutes")

#### Scenario: Past timestamp

- **WHEN** the component is given a timestamp earlier than the current time
- **THEN** it renders a backward-looking phrase (e.g. "12 minutes ago"), unchanged from previous behavior

#### Scenario: Existing past-timestamp call sites

- **WHEN** task start/finish times or an app's last-started time are rendered
- **THEN** their output is unchanged by the sign-aware behavior
