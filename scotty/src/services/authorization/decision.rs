//! The single authorization decision: may a user perform an action on a
//! target? Every check and enforcement point goes through [`Grants::decide`].

use std::collections::HashMap;

use super::policy::{Grants, Policy, ScopeGrant};
use super::types::Permission;

/// App name -> the scopes the app belongs to.
pub type AppScopes = HashMap<String, Vec<String>>;

/// What the user wants to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action<'a> {
    Permission(Permission),
    /// Create an app from this blueprint: allowed by `create` or by
    /// `create:<blueprint>`.
    CreateFrom(&'a str),
}

impl From<Permission> for Action<'_> {
    fn from(permission: Permission) -> Self {
        Self::Permission(permission)
    }
}

impl<'a> Action<'a> {
    /// `create`, or creating from `blueprint` when the request qualifies for
    /// `create:<blueprint>`.
    pub fn create(blueprint: Option<&'a str>) -> Self {
        blueprint.map_or(Self::Permission(Permission::Create), Self::CreateFrom)
    }

    /// The one refusal reason every enforcement point reports.
    fn denial(self) -> String {
        match self {
            Self::Permission(p) => format!("user lacks {} permission", p.as_str()),
            Self::CreateFrom(bp) => format!("user lacks create permission (or create:{bp})"),
        }
    }
}

/// Where the action applies.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// An app, through the scopes it belongs to. An app in no scope grants
    /// nothing.
    App(&'a str),
    /// At least one of these scopes (create, MCP scope targets).
    AnyOf(&'a [String]),
    /// Every one of these scopes, and at least one (replacing a multi-scope
    /// app).
    EveryOf(&'a [String]),
    /// Any scope the user holds (`admin_*`, cross-app operations).
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// Refused, with the reason to report.
    Deny(String),
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

impl ScopeGrant {
    pub fn allows(&self, action: Action<'_>) -> bool {
        match action {
            Action::Permission(p) => self.permissions.contains(p),
            Action::CreateFrom(bp) => {
                self.permissions.contains(Permission::Create) || self.blueprints.contains(bp)
            }
        }
    }
}

impl Grants {
    fn allows_in(&self, scope: &str, action: Action<'_>) -> bool {
        self.scopes().get(scope).is_some_and(|g| g.allows(action))
    }

    /// Decide `action` on `target` for the user these grants belong to.
    pub fn decide(&self, apps: &AppScopes, action: Action<'_>, target: Target<'_>) -> Decision {
        let allowed = match target {
            Target::App(app) => apps
                .get(app)
                .is_some_and(|scopes| scopes.iter().any(|s| self.allows_in(s, action))),
            Target::AnyOf(scopes) => scopes.iter().any(|s| self.allows_in(s, action)),
            Target::EveryOf(scopes) => {
                !scopes.is_empty() && scopes.iter().all(|s| self.allows_in(s, action))
            }
            Target::Global => self.scopes().values().any(|g| g.allows(action)),
        };
        if allowed {
            Decision::Allow
        } else {
            Decision::Deny(action.denial())
        }
    }
}

impl Policy {
    /// Decide `action` on `target` for `user`.
    pub fn decide(
        &self,
        apps: &AppScopes,
        user: &str,
        action: Action<'_>,
        target: Target<'_>,
    ) -> Decision {
        self.grants_for(user).decide(apps, action, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::policy::tests::config;

    const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
  beta: { description: Beta, created_at: 2024-01-01T00:00:00Z }
roles:
  dev: { permissions: [view, manage, create], description: list }
  viewer: { permissions: [view], description: view }
  publisher: { permissions: ['create:static-html'], description: blueprint }
assignments:
  ann@example.com:
    - { role: dev, scopes: [alpha] }
    - { role: viewer, scopes: [beta] }
  identifier:ci:
    - { role: publisher, scopes: [alpha] }
"#;

    const ANN: &str = "ann@example.com";
    const CI: &str = "identifier:ci";

    fn policy() -> Policy {
        Policy::compile(config(POLICY)).unwrap()
    }

    fn apps() -> AppScopes {
        AppScopes::from([
            ("app-a".into(), vec!["alpha".into()]),
            ("app-ab".into(), vec!["alpha".into(), "beta".into()]),
            ("app-none".into(), vec![]),
            // An app named like a scope is still only in its own scopes.
            ("beta".into(), vec!["alpha".into()]),
        ])
    }

    fn scopes(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    fn allowed(user: &str, action: Action<'_>, target: Target<'_>) -> bool {
        policy().decide(&apps(), user, action, target).is_allowed()
    }

    const MANAGE: Action<'static> = Action::Permission(Permission::Manage);
    const VIEW: Action<'static> = Action::Permission(Permission::View);

    #[test]
    fn app_target_uses_the_app_scopes() {
        assert!(allowed(ANN, MANAGE, Target::App("app-a")));
        assert!(allowed(ANN, MANAGE, Target::App("app-ab")));
        assert!(allowed(ANN, VIEW, Target::App("beta")));
        assert!(!allowed(
            ANN,
            Action::Permission(Permission::Destroy),
            Target::App("app-a")
        ));
    }

    #[test]
    fn app_in_no_scope_grants_nothing() {
        assert!(!allowed(ANN, VIEW, Target::App("app-none")));
        assert!(!allowed(ANN, VIEW, Target::App("unknown")));
        // Named like a scope the user holds, but not registered.
        assert!(!allowed(ANN, VIEW, Target::App("alpha")));
    }

    #[test]
    fn any_of_needs_one_scope() {
        let both = scopes(&["beta", "alpha"]);
        assert!(allowed(ANN, MANAGE, Target::AnyOf(&both)));
        assert!(!allowed(ANN, MANAGE, Target::AnyOf(&scopes(&["beta"]))));
        assert!(!allowed(ANN, VIEW, Target::AnyOf(&[])));
    }

    #[test]
    fn every_of_needs_every_scope_and_at_least_one() {
        assert!(allowed(
            ANN,
            VIEW,
            Target::EveryOf(&scopes(&["alpha", "beta"]))
        ));
        assert!(!allowed(
            ANN,
            MANAGE,
            Target::EveryOf(&scopes(&["alpha", "beta"]))
        ));
        assert!(!allowed(ANN, VIEW, Target::EveryOf(&[])));
    }

    #[test]
    fn global_means_in_any_scope() {
        assert!(allowed(ANN, MANAGE, Target::Global));
        assert!(!allowed(
            ANN,
            Action::Permission(Permission::AdminRead),
            Target::Global
        ));
        assert!(!allowed("nobody@example.com", VIEW, Target::Global));
    }

    #[test]
    fn create_from_blueprint_is_allowed_by_create_or_the_blueprint_grant() {
        let alpha = scopes(&["alpha"]);
        let target = Target::AnyOf(&alpha);
        assert!(allowed(ANN, Action::CreateFrom("anything"), target));
        assert!(allowed(CI, Action::CreateFrom("static-html"), target));
        assert!(!allowed(CI, Action::CreateFrom("other"), target));
        assert!(!allowed(CI, Action::Permission(Permission::Create), target));
        assert!(allowed(
            CI,
            Action::CreateFrom("static-html"),
            Target::Global
        ));
    }

    #[test]
    fn deny_reason_names_the_permission() {
        let deny = |action| policy().decide(&apps(), "nobody@example.com", action, Target::Global);
        assert_eq!(
            deny(MANAGE),
            Decision::Deny("user lacks manage permission".into())
        );
        let Decision::Deny(reason) = deny(Action::CreateFrom("static-html")) else {
            panic!("allowed");
        };
        assert!(reason.contains("lacks create permission"), "{reason}");
        assert!(reason.contains("create:static-html"), "{reason}");
    }

    #[test]
    fn permission_converts_into_an_action() {
        assert_eq!(Action::from(Permission::View), VIEW);
    }
}
