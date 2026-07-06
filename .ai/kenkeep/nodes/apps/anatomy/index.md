# kenkeep Index: apps / anatomy

↑ Parent: [apps](../index.md)

> kenkeep navigation: the injected body above is the root index node, the top-level catalog of branches and root-level leaves. Do not expect the whole knowledge base here; descend on demand. Read the root index node, pick one or more branches whose intent and tags match your task (several branches can be relevant), and read those branch `index.md` nodes. Descend further only where the task needs it, opening only the leaves you have confirmed are relevant. Follow each leaf's `relates_to` and `depends_on` cross edges to reach related leaves in other branches. You decide how deep to go per branch.

> This index only orients you; leaves hold the durable guidance. Open at least one relevant leaf before acting.

## Subfolders
_None._

## Conventions (how we build)
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) to learn about: Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml. #scotty #compose #file-ownership

## Components (what exists)
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) to learn about: Apps are identified by folder name; service hostnames derive from app name + service name unless overridden. #scotty #apps #vocabulary
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) to learn about: App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`. #authorization #scopes #config

## By topic

### #scotty
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](../lifecycle/map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
- Open [**compose.yml must not expose ports directly or use env-var expansion**](../lifecycle/practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
### #apps
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](../lifecycle/map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.
### #authorization
- Open [**Casbin assignment matching follows exact > domain > wildcard precedence**](../../auth/authorization/practice-casbin-assignment-precedence.md) — Exact email match beats domain pattern beats wildcard; wildcard assignments are always additive.
- Open [**Casbin policy assignment keys are formatted differently per auth_mode**](../../auth/authorization/map-authorization-assignment-key-format-by-auth-mode.md) — OAuth mode keys assignments by email; bearer mode uses 'identifier:token_name'; dev mode has no applicable assignments.
- Open [**Authorization uses Casbin RBAC**](../../auth/authorization/map-authorization-casbin-rbac.md) — RBAC via Casbin; config at config/casbin/policy.yaml, impl in services/authorization/casbin.rs.
### #compose
- Open [**compose.yml must not expose ports directly or use env-var expansion**](../lifecycle/practice-unsupported-compose-features.md) — Scotty marks apps unsupported if compose.yml exposes ports directly or uses environment-variable expansion.
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #config
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
- Open [**Enable metrics/traces export via SCOTTY__TELEMETRY**](../../configuration/practice-enable-telemetry-env-var.md) — Set SCOTTY__TELEMETRY=metrics,traces (or just metrics/traces) to have Scotty export OTLP telemetry.
- Open [**Scotty Docker image ships binaries and non-sensitive config only**](../../configuration/map-docker-image-excludes-secrets.md) — The official Docker image bundles binaries, Casbin model, and blueprints, but no secrets — those are supplied at runtime.
### #file-ownership
- Open [**Scotty only writes compose.override.yml and .scotty.yml in an app directory**](practice-scotty-only-touches-override-and-settings-files.md) — Scotty must not modify any file in an app's directory besides compose.override.yml and .scotty.yml.
### #scopes
- Open [**Apps declare authorization scopes in .scotty.yml**](map-app-scope-declaration-in-scotty-yml.md) — App-to-scope membership is set via a \`scopes:\` list in .scotty.yml; unset apps land in \`default\`.
### #vocabulary
- Open [**An app is any folder in the apps directory containing compose.yml**](map-app-anatomy.md) — Apps are identified by folder name; service hostnames derive from app name + service name unless overridden.
- Open [**Apps are categorized as owned, supported, or unsupported**](../lifecycle/map-app-types-owned-supported-unsupported.md) — Scotty validates compose.yml and sorts apps into owned/supported/unsupported, each with different lifecycle guarantees.