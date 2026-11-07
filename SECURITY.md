# Security Policy

## Supported versions

Security fixes are released for the latest minor version of Scotty. Older
versions do not receive fixes; please upgrade.

| Version | Supported |
|---------|-----------|
| latest `0.x` minor | yes |
| older | no |

## Reporting a vulnerability

Please do **not** open a public issue, pull request or discussion for a
security problem.

Report it privately through GitHub:
[Report a vulnerability](https://github.com/factorial-io/scotty/security/advisories/new).

Include what you can of:

- the affected component (server, `scottyctl`, frontend, Docker image) and version
- steps to reproduce, or a proof of concept
- the impact you expect (for example: access to another scope's apps, code
  execution on the Docker host, leaked credentials)

## What to expect

- We acknowledge your report within 5 working days.
- We confirm or rule out the issue and share a first severity assessment
  within 14 days.
- Fix targets by severity: critical within 7 days, high within 30 days,
  medium within 90 days.
- We publish a GitHub Security Advisory with the fixed release and credit you,
  unless you prefer to stay anonymous.

## Scope

In scope: the `scotty` server, `scottyctl`, the bundled frontend, and the
published Docker image.

Expected behaviour, not a vulnerability: users with the `create` permission
can deploy Docker Compose files that control the Docker host (privileged
containers, host mounts). Treat `create` as host-level trust. See
[Security](https://scotty.factorial.io/security.html) in the
documentation for the trust model and the automated checks.
