## ADDED Requirements

### Requirement: Autowake app setting
Apps SHALL have an `autowake` boolean setting that defaults to `false`. Settings stored without the field SHALL load with `autowake: false`.

#### Scenario: Existing app without the field
- **WHEN** an app's stored settings do not contain `autowake`
- **THEN** the app loads with `autowake` set to `false` and behaves as before

#### Scenario: scottyctl sets autowake
- **WHEN** a user runs `scottyctl app:create` with `--autowake` and `--basic-auth user:pass`
- **THEN** the create request carries `autowake: true` in the app settings

### Requirement: Autowake requires basic auth at creation
The server SHALL reject an app creation request that sets `autowake: true` without `basic_auth`, and SHALL NOT create the app.

#### Scenario: Autowake without basic auth
- **WHEN** a create request has `autowake: true` and `basic_auth` unset
- **THEN** the server responds with a 400 error stating that autowake requires basic auth

#### Scenario: Autowake with basic auth
- **WHEN** a create request has `autowake: true` and `basic_auth` set
- **THEN** the app is created with `autowake: true`

### Requirement: Credentials challenge for stopped autowake apps
When a request for the domain of a `Stopped` app that has `autowake` and `basic_auth` set arrives at Scotty's catch-all handler without matching basic-auth credentials, Scotty SHALL respond `401` with header `WWW-Authenticate: Basic realm="traefik"` and no-cache headers, and SHALL NOT start the app.

#### Scenario: No Authorization header
- **WHEN** a request for a stopped autowake app's domain has no `Authorization` header
- **THEN** Scotty responds 401 with `WWW-Authenticate: Basic realm="traefik"` and the app stays stopped

#### Scenario: Wrong credentials
- **WHEN** the request carries a `Basic` header whose user or password does not match the app's `basic_auth`
- **THEN** Scotty responds 401 and the app stays stopped

#### Scenario: Malformed header
- **WHEN** the `Authorization` header is not valid `Basic` base64 `user:password`
- **THEN** Scotty responds 401 and the app stays stopped

### Requirement: Wake on valid credentials
When the credentials match, Scotty SHALL start the app in the background without a Scotty login and without sending the `AppStarted` notification. It SHALL respond `503` with an HTML page that states the app is starting and refreshes itself after 5 seconds, a `Retry-After: 5` header, and no-cache headers. Any HTTP method SHALL trigger the wake.

#### Scenario: First valid request
- **WHEN** a request with matching credentials hits a stopped autowake app's domain
- **THEN** Scotty starts the app and responds 503 with a self-refreshing "starting" page

#### Scenario: No notification on wake
- **WHEN** an app is started by autowake and the start succeeds
- **THEN** no `AppStarted` notification is sent to the app's notification receivers

#### Scenario: Non-GET request
- **WHEN** a POST request with matching credentials hits a stopped autowake app's domain
- **THEN** Scotty starts the app and responds with the same 503 page

### Requirement: Single start per app
Scotty SHALL start a given app at most once at a time through autowake. Requests that arrive while a wake for that app is in progress SHALL receive the 503 starting page without triggering another start. After the wake operation ends, successfully or not, a new request MAY trigger a new wake.

#### Scenario: Concurrent requests
- **WHEN** several requests with valid credentials arrive for the same stopped autowake app before its containers exist
- **THEN** exactly one start operation runs and every request receives the 503 starting page

#### Scenario: Wake after failed start
- **WHEN** an autowake start fails and the app is still stopped
- **THEN** the next valid request triggers a new start

### Requirement: Fallback for autowake without basic auth
A stopped app with `autowake: true` but no `basic_auth` SHALL be handled exactly like a stopped app without autowake (redirect to the landing page), and SHALL NOT be started by the request.

#### Scenario: Basic auth missing at runtime
- **WHEN** a request hits the domain of a stopped app whose settings have `autowake: true` and no `basic_auth`
- **THEN** Scotty responds with the existing 302 redirect to the landing page and does not start the app

#### Scenario: Autowake disabled
- **WHEN** a request hits the domain of a stopped app with `autowake: false`
- **THEN** Scotty responds with the existing 302 redirect to the landing page
