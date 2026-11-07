# app-action-feedback

## MODIFIED Requirements

### Requirement: Refused or failed action requests are reported

When an action request to the server fails (any non-2xx response or network error), the frontend SHALL leave the action controls usable again and SHALL display the failure in the global error dialog, including the server's message when the response carries one. When an accepted action's task later fails, the frontend SHALL show the failure on the action control itself.

#### Scenario: Server refuses the action
- **WHEN** the user triggers an action and the server responds 403 with `{ "error": true, "message": "..." }`
- **THEN** the busy indicator is cleared
- **AND** the message from the response is shown in the global error dialog

#### Scenario: Response without a message
- **WHEN** the server responds with a non-2xx status and no JSON message
- **THEN** the status code and status text are shown in the global error dialog instead

#### Scenario: Accepted task fails later
- **WHEN** the server accepts a run or stop request and the resulting task fails
- **THEN** the action control shows an error state with the failure available as a tooltip
