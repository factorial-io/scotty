## Purpose

Defines which app actions the web frontend offers for a given app and how a refused or failed action request is reported to the user, so that the UI never shows an action the server will refuse for that app and never fails silently.

## Requirements

### Requirement: Action availability follows the app's scopes

The frontend SHALL offer an app action (run, stop, rebuild, purge, destroy, custom action, shell, logs) only if the user holds the required permission in at least one of the app's scopes. An app's scopes are its `settings.scopes`; an app without settings belongs to the `default` scope. Global (non-app) permission checks SHALL keep granting when the permission is held in any scope.

#### Scenario: Permission held only in another scope
- **WHEN** the user has `manage` in scope `client-a` and `view` in scope `default`
- **AND** the app has no settings
- **THEN** the app detail page and the app list show no Run, Stop, Rebuild or Purge controls for that app

#### Scenario: Permission held in the app's scope
- **WHEN** the user has `manage` in scope `client-a`
- **AND** the app declares `scopes: [client-a]`
- **THEN** the Run, Stop, Rebuild and Purge controls are shown

#### Scenario: Dev auth mode
- **WHEN** the server runs with auth mode `dev`
- **THEN** every action is offered regardless of scopes

### Requirement: Refused or failed action requests are reported

When an action request to the server fails (any non-2xx response or network error), the frontend SHALL leave the action controls usable again and SHALL display the failure to the user, including the server's message when the response carries one.

#### Scenario: Server refuses the action
- **WHEN** the user triggers an action and the server responds 403 with `{ "error": true, "message": "..." }`
- **THEN** the busy indicator is cleared
- **AND** the message from the response is shown next to the action controls

#### Scenario: Response without a message
- **WHEN** the server responds with a non-2xx status and no JSON message
- **THEN** the status code and status text are shown instead

### Requirement: Authorization refusals carry a message

The server's authorization middleware SHALL answer a refused request with status 403 and the standard error body `{ "error": true, "message": <reason> }`, where the reason names the missing permission.

#### Scenario: Missing app permission
- **WHEN** a request for an app requires `manage` and the user lacks it for that app's scopes
- **THEN** the response is 403 with a JSON body whose `message` contains `lacks manage permission`
