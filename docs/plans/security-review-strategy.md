# Security Review and Penetration Testing Strategy

Status: proposal
Scope: `scotty` server, `scottyctl`, `frontend`, the shipped Docker image, and a
running deployment (Traefik, Docker host, OAuth provider integration).

This document has two parts:

1. **Prior art.** Which established methods this plan draws on.
2. **The plan for Scotty.** Threat model, a phased review (static, then
   manual, then dynamic), a test matrix for a running server, and how findings
   become regression tests.

---

## 1. Prior art: established approaches and where they disagree

| Approach | Core idea | Key reference |
|---|---|---|
| **Threat modeling (STRIDE, data flow diagrams)** | Draw the system, mark trust boundaries, ask "what can go wrong" at each one. | Adam Shostack, *Threat Modeling: Designing for Security* (2014); Microsoft STRIDE |
| **Attack trees** | Start from an attacker goal ("root on the Docker host") and break it down into ways to reach it. | Bruce Schneier, "Attack Trees", Dr. Dobb's (1999) |
| **Risk-centric modeling (PASTA)** | Threat modeling tied to business impact; seven stages from business goals down to attack simulation. | UcedaVélez & Morana, *Risk Centric Threat Modeling* (2015) |
| **Verification standards (checklist)** | A fixed requirement catalogue to verify against, in levels L1 to L3. | OWASP **ASVS 5.0**; OWASP **API Security Top 10 (2023)** |
| **Test methodology** | A test case catalogue for web apps and APIs. | OWASP **Web Security Testing Guide (WSTG) v4.2** |
| **Pentest process frameworks** | Engagement phases: pre-engagement, recon, threat modeling, exploitation, post-exploitation, reporting. | **PTES**; **NIST SP 800-115**; OSSTMM 3 |
| **Container/host hardening** | Benchmarks for the Docker daemon, images and runtime. | **CIS Docker Benchmark**; NIST SP 800-190 (Application Container Security) |
| **Supply chain** | Dependency and build provenance. | RustSec advisory DB, `cargo-audit` / `cargo-deny`; SLSA; OpenSSF Scorecard |

**Where they disagree:**

- **Shostack and Schneier** argue that a checklist without a model of the
  system finds generic bugs and misses the architectural ones. In Scotty, the
  biggest risk is architectural: anyone who can submit a `docker-compose.yml`
  can effectively control the Docker host. No OWASP checklist item covers that.
- **The ASVS/WSTG camp** argues that threat models depend on who draws them,
  while a standard makes coverage measurable and repeatable. It also catches
  the boring bugs (missing security headers, broken session expiry) that
  threat-modeling workshops skip.
- **Practitioners (PTES, red-team culture)** argue that only an adversarial
  test against a real deployment shows what is exploitable: misconfigured
  Traefik, leaked tokens in logs, a Docker socket exposed on the network. Code
  review cannot see the deployment.
- **The supply-chain view (SLSA, Scorecard)** points out that many recent
  incidents came through dependencies and CI, not application logic.

**Further reading:**

- Shostack, *Threat Modeling* (Wiley 2014), the practical STRIDE handbook
- OWASP ASVS 5.0 (2025): chapters V1 (encoding), V6 (authentication), V8
  (authorization), V10 (OAuth/OIDC), V12 (secure communication), V13
  (configuration)
- OWASP WSTG v4.2, especially WSTG-ATHN, WSTG-ATHZ, WSTG-SESS and the WebSocket
  test case WSTG-CLNT-10
- OWASP API Security Top 10 2023: API1 BOLA, API5 BFLA and API3 BOPLA map
  directly onto Scotty's scope-based RBAC
- NIST SP 800-115, *Technical Guide to Information Security Testing*
- NIST SP 800-190, *Application Container Security Guide*
- Liz Rice, *Container Security* (O'Reilly, 2nd ed.), for the Docker-host
  attack paths
- *The Web Application Hacker's Handbook* (Stuttard/Pinto). Dated, but still
  the best explanation of how to think about access control testing.

### Synthesis (my recommendation)

Use all four approaches, each where it is strongest:

1. A **threat model and attack trees** decide *where* to look.
2. **ASVS L2** gives a coverage baseline and keeps the review honest.
3. **Automated SAST/SCA/container scans** run continuously in CI.
4. A **time-boxed pentest following PTES/WSTG** runs against a staging
   deployment that mirrors production.
5. **Every confirmed finding becomes an integration test** in `scotty/tests/`
   before it is closed, so the review leaves a lasting regression suite and
   not just a PDF.

---

## 2. The Scotty plan

### 2.1 Assets and attacker goals

| Asset | Why it matters |
|---|---|
| Docker host (daemon access) | Scotty talks to the Docker daemon. Controlling Scotty usually means root on the host. |
| Secrets: bearer tokens, OAuth client secret, registry passwords, 1Password Connect token, app env vars (`op://` resolved) | Lateral movement into other systems |
| Other tenants' apps (scopes) | Scotty is multi-tenant by scope. Crossing scopes breaks client isolation. |
| Shell sessions / file transfer | Direct code execution and data access inside containers |
| Traefik routing | Hijacking hostnames, TLS, or the default-backend landing page |
| Audit trail / notifications | Covering tracks, spoofing notifications (webhook, Mattermost, GitLab) |

Attacker profiles:

- **A0, anonymous internet user.** Reaches public routes: `/api/v1/health`,
  `/api/v1/info`, `/ws`, `/oauth/*`, `/api/v1/login`, `/swagger-ui`,
  `/api-docs/openapi.json`, plus the landing page via Traefik.
- **A1, authenticated low-privilege user.** For example a `viewer` in scope
  `default`, or anyone matching the `'*'` wildcard assignment.
- **A2, developer in scope X.** Has `create`/`manage`/`shell` and attacks
  scope Y or the host.
- **A3, malicious app / compromised container.** Code running inside a
  deployed app that attacks Scotty, Traefik or sibling apps over the network.
- **A4, supply chain.** A compromised crate, npm package, base image or
  GitHub Action.

### 2.2 Trust boundaries (data flow)

```
 Internet ──► Traefik ──► Scotty HTTP/WS (axum)
                │              │  ├─ auth_core (bearer / OAuth / dev)
                │              │  ├─ Casbin RBAC (scopes, roles)
                │              │  └─ rate limiting (per instance)
                │              ├──► Docker daemon (bollard)  ◄── highest-value boundary
                │              ├──► filesystem apps/<name>/ (compose files, .env)
                │              ├──► OIDC provider, 1Password Connect, registries
                │              └──► notification targets (webhooks, Mattermost, GitLab)
                └──► deployed apps (per-app proxy networks)
```

### 2.3 Attack trees for the top three goals

**G1: root on the Docker host**
- Submit a compose file with `privileged: true`, `volumes: ["/:/host"]`,
  `/var/run/docker.sock`, `pid: host`, `network_mode: host`, `cap_add`, or
  `devices` through `app:create`. As far as this survey found, compose content
  is not checked against any policy, so the `create` permission is
  effectively root on the host.
- Path traversal in uploaded file names
  (`scotty/src/docker/steps/files.rs:43` uses a lexical `path_clean` plus a
  `starts_with` check). Test symlinks, absolute paths, `..` after
  normalisation, Unicode normalisation, and Windows-style separators.
- Decompression bombs in the gzip/base64 upload
  (`api/rest/handlers/apps/create.rs`): is `create_app_max_size` applied
  before or after decompression?
- Custom actions or blueprint actions that run commands: approval bypass,
  TOCTOU between approval and execution, command injection through
  `service:command` parsing.
- `app:adopt` of an existing compose project outside the apps root.

**G2: act in another tenant's scope (BOLA/BFLA)**
- Every `{app}` path parameter: is the scope check done against the stored
  app's scope, or against something the client supplies?
- WebSocket messages that carry IDs (`session_id`, `stream_id`, `task_id`)
  without checking ownership. Known case: `ShellSessionData`,
  `ResizeShellTty` and `TerminateShellSession` in
  `scotty/src/api/websocket/handlers/shell.rs`. These are partly tracked in
  bean `scotty-qaby`, but `ShellSessionData` (shell **input**) is broader than
  the bean describes; see 2.6.
- Task output (`/api/v1/authenticated/tasks`, `StartTaskOutputStream`): can
  one user read another user's task output, which may contain secrets?
- Admin endpoints (`admin/*`): can any non-admin path reach them?
- Assignment precedence edge cases: case folding, `@sub.factorial.io` versus
  `@factorial.io`, trailing dots, Unicode lookalikes, plus-addressing.

**G3: steal credentials**
- Secrets in logs and traces (OTEL spans, `RUST_LOG=debug`, task output,
  error messages).
- OAuth: `redirect_uri` and open redirects in `/oauth/authorize`,
  `/api/oauth/callback` and the landing page; state/PKCE handling; replay of
  `/oauth/exchange` codes; device-code brute force or phishing.
- `/api/v1/info` and the OpenAPI docs: what do they disclose
  unauthenticated?
- Default config: `config/default.yaml` ships sample bearer tokens and
  registry credentials. Check that production deployments override them, and
  consider refusing to start when known sample values are present.
- Frontend: XSS anywhere app names, log lines, env vars or notification text
  are rendered leads directly to bearer-token theft if the token is stored in
  `localStorage`.

### 2.4 Phased plan

#### Phase 0: preparation (0.5 day)
- Freeze a commit and image digest for the review.
- Set up a **staging environment that mirrors production**: same Traefik
  config, same auth mode (OAuth plus bearer), real OIDC test tenant,
  1Password test vault, and a dedicated Docker host that can be destroyed.
  **Never pentest the production Docker host**: G1 tests are destructive by
  nature.
- Create test identities for each attacker profile: `viewer@`, `dev-a@`
  (scope A), `dev-b@` (scope B), `approver@`, `admin@`, an unmatched external
  domain, and one bearer token per role.
- Write the rules of engagement: written authorization from the system owner,
  time window, out-of-scope systems (the OIDC provider itself, the shared
  registry), an emergency contact, and a data-handling rule for any secrets
  found.
- Re-check the open security beans under epic `scotty-uq4t` to see which are
  still valid.

#### Phase 1: automated baseline (1 day; afterwards it runs in CI)

| Area | Tool | Command / note |
|---|---|---|
| Rust advisories | `cargo-audit` | `cargo audit` |
| Licences, bans, duplicate and yanked crates | `cargo-deny` | add `deny.toml`, `cargo deny check` |
| Rust lints with security relevance | `clippy` | add `-W clippy::unwrap_used -W clippy::indexing_slicing` on handler modules |
| `unsafe` inventory | `cargo-geiger` | expect near zero in first-party crates |
| SAST | `semgrep` (`p/rust`, `p/secrets`), GitHub CodeQL (Rust + JS/TS) | |
| Frontend dependencies | `bun audit` / `osv-scanner` | `osv-scanner --lockfile=frontend/bun.lock` |
| Secrets in history | `gitleaks` / `trufflehog` | `gitleaks detect --source .` |
| Container image | `trivy image`, `grype`, `dockle` | scan the released image digest |
| Dockerfile / compose | `hadolint`, `trivy config` | |
| GitHub Actions | `zizmor`, OpenSSF Scorecard | pinned actions, `pull_request_target` misuse, token permissions; `claude*.yml` workflows deserve attention |

Output: a triaged list. False positives are documented, real issues become
beans.

#### Phase 2: manual code review (3–5 days), guided by the attack trees

_Not yet written. Remaining sections still to come: Phase 2 (manual code
review), Phase 3 (dynamic testing on staging), reporting and regression
tests._