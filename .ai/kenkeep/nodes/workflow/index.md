# kenkeep Index: workflow

↑ Parent: [kenkeep](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Git conventions for this repo**](practice-git-rules.md) to learn about: Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits. #git #conventions
- Open [**Contributor setup instructions belong in the root README**](practice-contributor-setup-instructions-belong-in-the-root-readme.md) to learn about: Per-clone setup steps go in the root README's Developing/Contributing section, not in subdirectory READMEs. #documentation #readme #conventions
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) to learn about: The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free. #vcs #jj #git-hooks #workflow
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) to learn about: Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release. #release #versioning #ci
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) to learn about: Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking. #testing #conventions
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) to learn about: Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans. #project-management #beans #workflow

## Components (what exists)
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) to learn about: The project uses a pre-push git hook installed by cargo-husky, set up automatically. #git #tooling #hooks #husky

## By topic

### #conventions
- Open [**Git conventions for this repo**](practice-git-rules.md) — Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits.
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) — Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking.
- Open [**Contributor setup instructions belong in the root README**](practice-contributor-setup-instructions-belong-in-the-root-readme.md) — Per-clone setup steps go in the root README's Developing/Contributing section, not in subdirectory READMEs.
### #git
- Open [**Git conventions for this repo**](practice-git-rules.md) — Never delete frontend/build/.gitkeep; no emojis in commit messages; use conventional commits.
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
- Open [**Which files under config/ are committed vs git-ignored**](../configuration/map-config-directory-git-tracking.md) — config/*.example and casbin/model.conf are committed templates; default.yaml, local.yaml, and casbin/policy.yaml hold real values and are meant to stay out of git (policy.yaml only if it has no secrets).
### #workflow
- Open [**Custom actions require approval before execution**](../apps/map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.
### #beans
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
### #ci
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.
### #documentation
- Open [**Contributor setup instructions belong in the root README**](practice-contributor-setup-instructions-belong-in-the-root-readme.md) — Per-clone setup steps go in the root README's Developing/Contributing section, not in subdirectory READMEs.
### #git-hooks
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.
### #hooks
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #husky
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
### #jj
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.
### #project-management
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
### #readme
- Open [**Contributor setup instructions belong in the root README**](practice-contributor-setup-instructions-belong-in-the-root-readme.md) — Per-clone setup steps go in the root README's Developing/Contributing section, not in subdirectory READMEs.
### #release
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.
### #testing
- Open [**Test placement and tooling conventions**](practice-testing-conventions.md) — Unit tests are colocated with implementation; integration tests live in scotty/tests; axum-test and wiremock are used for HTTP/mocking.
### #tooling
- Open [**Pre-push git hook installed via cargo-husky**](map-pre-push-hook-cargo-husky.md) — The project uses a pre-push git hook installed by cargo-husky, set up automatically.
- Open [**Frontend tooling uses bun, not npm**](../frontend/practice-frontend-uses-bun.md) — Frontend install/dev/build/check run via bun; lint (Prettier + ESLint) must pass before push.
### #vcs
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.
### #versioning
- Open [**Releases are fully automated via release-please**](practice-release-process-automation.md) — Never manually bump versions or edit the changelog; conventional-commit type drives the version bump; merging the standing release PR performs the release.