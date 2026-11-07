# Security

This page describes how security issues are reported and which automated
security checks run on the Scotty repository.

## Reporting a vulnerability

Please do not open a public issue for security problems. Report them
privately through
[GitHub Security Advisories](https://github.com/factorial-io/scotty/security/advisories/new).
See [`SECURITY.md`](https://github.com/factorial-io/scotty/blob/main/SECURITY.md)
for supported versions and response times.

## Trust model

Keep these properties in mind when deploying Scotty:

- **The `create` permission is host-level trust.** Scotty runs the Docker
  Compose files users submit. A compose file can request privileged
  containers, host mounts or the Docker socket, so anyone allowed to create
  apps can effectively control the Docker host. Grant `create` only to people
  you would give shell access to the host.
- **Shell sessions and log streams belong to one WebSocket connection.** Only
  the connection that started a session can send input to it, resize it or end
  it. Sessions end when the connection closes.
- **Rate limits apply per instance.** When running several Scotty instances
  behind a load balancer, each one counts separately.
- **Override all sample credentials.** `config/default.yaml` ships sample
  bearer tokens and registry passwords for local development. Set real values
  through `SCOTTY__` environment variables in every deployment.

## Automated checks

The `Security` workflow (`.github/workflows/security.yml`) runs on every pull
request, on every push to `main`, and weekly on Mondays.

| Job | Tool | Fails when |
|---|---|---|
| Rust dependencies | `cargo deny` | a crate has a RustSec advisory, is yanked, has a license outside the allow-list in `deny.toml`, or comes from an unknown registry |
| Frontend dependencies | `osv-scanner` | `frontend/bun.lock` or `docs/bun.lock` contains a package with a vulnerability of CVSS 7.0 or higher (currently report-only) |
| Secrets | `gitleaks` | the commits of a pull request contain a credential (the weekly run scans the whole history) |
| Workflow definitions | `zizmor` | a GitHub workflow has a high-severity finding, e.g. an action not pinned to a commit SHA |
| Exception expiry | `check-security-exceptions.py` | an accepted exception has passed its expiry date or has none |
| Published image | `trivy` | weekly: the published `main` image has a fixable high or critical vulnerability |

The `Build and Push Docker Image` workflow also scans each image it builds
with `trivy`, on pull requests and on pushes.

### Running the checks locally

```bash
cargo install cargo-deny --locked
cargo deny check advisories bans licenses sources

go run github.com/zricethezav/gitleaks/v8@v8.30.1 dir --config .gitleaks.toml .
go run github.com/google/osv-scanner/v2/cmd/osv-scanner@v2.7.0 scan source \
  --lockfile=frontend/bun.lock --lockfile=docs/bun.lock

pipx run zizmor --offline --min-severity high .github/workflows
python3 .github/scripts/check-security-exceptions.py
```

### Accepting an exception

Fix the finding whenever possible: update the dependency, rotate the secret,
or change the workflow. If a finding does not apply (for example, the
vulnerable code path is never used), record an exception with a reason and an
expiry date of at most one year:

- **Rust advisory**: add an entry to `[advisories] ignore` in `deny.toml`,
  one per line:
  `{ id = "RUSTSEC-2026-0001", reason = "only used in tests. expires: 2027-06-30" },`
- **Container image**: add the ID to `.trivyignore` with a comment on the line
  above: `# not reachable from scotty. expires: 2027-06-30`
- **Workflow finding**: add an inline suppression on the flagged line:
  `# zizmor: ignore[<audit>] <reason>, expires: 2027-06-30`
- **Secret false positive**: add a narrow pattern, path or commit to the
  allow-list in `.gitleaks.toml`. Never allow-list a real credential; rotate
  it instead.

When the date passes, the exception expiry job fails and the exception has to
be reviewed again.

All third-party actions are pinned to a commit SHA, with the version tag in a
comment. Renovate keeps the pins up to date.
