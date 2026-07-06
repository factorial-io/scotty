# kenkeep Index: apps / custom-actions

↑ Parent: [apps](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
_None yet._

## Components (what exists)
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) to learn about: run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes. #custom-actions #authorization #blueprints
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) to learn about: Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions. #custom-actions #workflow #security

## By topic

### #custom-actions
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../../auth/authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../../auth/authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](../../auth/authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #blueprints
- Open [**Blueprints are reusable app templates**](../blueprints/map-blueprints-concept.md) — Templates defining required/public services, port mappings, lifecycle actions, and per-service custom actions.
- Open [**Blueprint action scripts get SCOTTY__APP_NAME and SCOTTY__PUBLIC_URL__<SERVICE> injected**](../blueprints/map-blueprint-injected-env-vars.md) — Scotty auto-injects the app name and each public service's URL as env vars into blueprint action commands.
- Open [**Custom action execution checks per-app actions before blueprint actions and always gates on can_execute()**](map-custom-action-execution-lookup-and-gate.md) — run_custom_action_handler looks up per-app custom actions first, falls back to blueprint actions, and only runs an action if CustomAction::can_execute() (approval status + expiration) passes.
### #security
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../../auth/authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Default-backend landing page security properties**](../../traefik/map-default-backend-security-model.md) — The landing-page redirect flow validates return_url against the app's own domain and still enforces normal manage permission to start the app.
- Open [**Authorization uses Casbin RBAC**](../../auth/authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #workflow
- Open [**Custom actions require approval before execution**](map-custom-actions-approval-workflow.md) — Actions move Pending -> Approved (or Rejected/Revoked/Expired); only Approved actions can run, gated by 4 dedicated permissions.
- Open [**Use beans CLI for issue tracking, not ad hoc todo lists**](../../workflow/practice-project-management-beans.md) — Beans is the agentic-first issue tracker; the .beans/ directory is committed; agents should track work via beans.
- Open [**Primary VCS workflow is jj; never depend on git commit hooks**](../../workflow/practice-primary-vcs-workflow-is-jj-never-depend-on-git-commit-hooks.md) — The maintainer drives this repo with jj, so git commit/pre-commit hooks never fire; tooling must work hook-free.