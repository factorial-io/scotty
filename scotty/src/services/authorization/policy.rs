//! The compiled authorization policy.
//!
//! [`Policy::compile`] turns an [`AuthConfig`] (the shape of `policy.yaml`)
//! into an immutable snapshot: roles resolved to permission sets, assignments
//! checked against the defined roles and scopes, and scope `*` expanded
//! against the scope list. Compiling is the only place the policy is
//! validated, so startup and runtime changes cannot disagree.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::types::{AuthConfig, Permission, PermissionOrWildcard};

/// Why a policy does not compile. Every variant names the offending key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PolicyError {
    #[error("role '{role}' has invalid permission '{permission}'")]
    InvalidPermission { role: String, permission: String },
    #[error("assignment for '{user}' names undefined role '{role}'")]
    UndefinedRole { user: String, role: String },
    #[error("assignment for '{user}' (role '{role}') names undefined scope '{scope}'")]
    UndefinedScope {
        user: String,
        role: String,
        scope: String,
    },
    #[error("Scope '{0}' already exists")]
    DuplicateScope(String),
    #[error("Role '{0}' already exists")]
    DuplicateRole(String),
}

/// A set of [`Permission`]s, one bit per variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PermSet(u16);

impl PermSet {
    pub fn all() -> Self {
        Permission::all().into_iter().collect()
    }

    pub fn contains(self, permission: Permission) -> bool {
        self.0 & Self::bit(permission) != 0
    }

    pub fn insert(&mut self, permission: Permission) {
        self.0 |= Self::bit(permission);
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn bit(permission: Permission) -> u16 {
        1 << permission as u16
    }
}

impl FromIterator<Permission> for PermSet {
    fn from_iter<I: IntoIterator<Item = Permission>>(iter: I) -> Self {
        let mut set = Self::default();
        for permission in iter {
            set.insert(permission);
        }
        set
    }
}

/// What a role grants in each scope it is assigned to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RoleGrant {
    pub(crate) permissions: PermSet,
    /// Blueprints of `create:<blueprint>` entries.
    pub(crate) blueprints: BTreeSet<String>,
    /// The role's permissions as written in the policy, for listings.
    pub(crate) listed: Vec<String>,
}

impl RoleGrant {
    fn compile(name: &str, permissions: &[PermissionOrWildcard]) -> Result<Self, PolicyError> {
        let mut grant = Self::default();
        for permission in permissions {
            let listed = permission.as_policy_string();
            // Roles built in code bypass the parser; hold them to its rules.
            if PermissionOrWildcard::parse(&listed).as_ref() != Some(permission) {
                return Err(PolicyError::InvalidPermission {
                    role: name.to_string(),
                    permission: listed,
                });
            }
            match permission {
                PermissionOrWildcard::Wildcard => grant.permissions = PermSet::all(),
                PermissionOrWildcard::Permission(p) => grant.permissions.insert(*p),
                PermissionOrWildcard::CreateFromBlueprint(bp) => {
                    grant.blueprints.insert(bp.clone());
                }
            }
            if !grant.listed.contains(&listed) {
                grant.listed.push(listed);
            }
        }
        Ok(grant)
    }
}

/// One assignment of a role, its scopes expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledAssignment {
    pub(crate) role: String,
    pub(crate) scopes: Vec<String>,
}

/// An immutable, validated authorization policy.
#[derive(Debug, Clone)]
pub struct Policy {
    /// The source the policy was compiled from, without app scopes.
    config: AuthConfig,
    roles: HashMap<String, RoleGrant>,
    /// Assignments by user pattern; email patterns lowercased.
    assignments: HashMap<String, Vec<CompiledAssignment>>,
}

impl Policy {
    /// Validate `config` and compile it. App scopes in `config.apps` are not
    /// part of the policy and are dropped.
    pub fn compile(mut config: AuthConfig) -> Result<Self, PolicyError> {
        config.apps.clear();

        let mut roles = HashMap::with_capacity(config.roles.len());
        for (name, role) in sorted(&config.roles) {
            roles.insert(name.clone(), RoleGrant::compile(name, &role.permissions)?);
        }

        let mut assignments: HashMap<String, Vec<CompiledAssignment>> = HashMap::new();
        for (user, entries) in sorted(&config.assignments) {
            for entry in entries {
                if !roles.contains_key(&entry.role) {
                    return Err(PolicyError::UndefinedRole {
                        user: user.clone(),
                        role: entry.role.clone(),
                    });
                }
                let scopes = expand_scopes(&config, user, &entry.role, &entry.scopes)?;
                assignments
                    .entry(normalize_user(user))
                    .or_default()
                    .push(CompiledAssignment {
                        role: entry.role.clone(),
                        scopes,
                    });
            }
        }

        Ok(Self {
            config,
            roles,
            assignments,
        })
    }

    /// The configuration this policy was compiled from (no app scopes).
    pub fn config(&self) -> &AuthConfig {
        &self.config
    }

    /// Everything `user` holds: the union of the assignments of their own
    /// key (email or identifier), their `@domain` and `*`. No match hides
    /// another.
    pub fn grants_for(&self, user: &str) -> Grants {
        let user = normalize_user(user);
        let mut keys = vec![user.as_str()];
        // `@domain` matches exactly that domain, never a subdomain.
        if let Some(at) = user.find('@') {
            keys.push(&user[at..]);
        }
        keys.push("*");
        keys.dedup();

        let mut grants = Grants::default();
        let matching = keys
            .iter()
            .filter_map(|key| self.assignments.get(*key))
            .flatten();
        for assignment in matching {
            // Compile guarantees the role exists.
            let Some(role) = self.roles.get(&assignment.role) else {
                continue;
            };
            for scope in &assignment.scopes {
                grants.scopes.entry(scope.clone()).or_default().add(role);
            }
        }
        grants
    }
}

/// What a user holds in one scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeGrant {
    pub permissions: PermSet,
    /// Blueprints the user may create from without plain `create`.
    pub blueprints: BTreeSet<String>,
    /// The permissions as written in the user's roles, for listings.
    pub listed: Vec<String>,
}

impl ScopeGrant {
    fn add(&mut self, role: &RoleGrant) {
        self.permissions = self.permissions.union(role.permissions);
        self.blueprints.extend(role.blueprints.iter().cloned());
        for permission in &role.listed {
            if !self.listed.contains(permission) {
                self.listed.push(permission.clone());
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.permissions.is_empty() && self.blueprints.is_empty()
    }
}

/// Everything one user holds, by scope. See [`Policy::grants_for`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Grants {
    scopes: BTreeMap<String, ScopeGrant>,
}

impl Grants {
    /// The user's grants by scope, scopes in name order.
    pub fn scopes(&self) -> &BTreeMap<String, ScopeGrant> {
        &self.scopes
    }

    /// Whether the user holds no permission in any scope.
    pub fn is_empty(&self) -> bool {
        self.scopes.values().all(ScopeGrant::is_empty)
    }
}

/// Scopes of an assignment with `*` replaced by every defined scope; every
/// other scope must be defined.
fn expand_scopes(
    config: &AuthConfig,
    user: &str,
    role: &str,
    scopes: &[String],
) -> Result<Vec<String>, PolicyError> {
    let mut expanded = BTreeSet::new();
    for scope in scopes {
        if scope == "*" {
            expanded.extend(config.scopes.keys().cloned());
        } else if config.scopes.contains_key(scope) {
            expanded.insert(scope.clone());
        } else {
            return Err(PolicyError::UndefinedScope {
                user: user.to_string(),
                role: role.to_string(),
                scope: scope.clone(),
            });
        }
    }
    Ok(expanded.into_iter().collect())
}

/// Emails compare case-insensitively (RFC 5321 in practice); identifiers and
/// other names are case-sensitive.
pub(crate) fn normalize_user(user: &str) -> String {
    if user.contains('@') {
        user.to_lowercase()
    } else {
        user.to_string()
    }
}

/// Map entries in key order, so errors are reported deterministically.
fn sorted<V>(map: &HashMap<String, V>) -> BTreeMap<&String, &V> {
    map.iter().collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(super) const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
  beta: { description: Beta, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: all }
  dev: { permissions: [view, manage, create, view], description: list }
  publisher: { permissions: ['create:static-html'], description: blueprint }
assignments:
  root@example.com:
    - { role: admin, scopes: ['*'] }
  Jane@Example.com:
    - { role: dev, scopes: [beta, alpha, beta] }
  ci:
    - { role: publisher, scopes: [] }
"#;

    pub(crate) fn config(yaml: &str) -> AuthConfig {
        serde_norway::from_str(yaml).unwrap()
    }

    fn compile_err(yaml: &str) -> PolicyError {
        Policy::compile(config(yaml)).unwrap_err()
    }

    #[test]
    fn wildcard_scope_expands_to_every_defined_scope() {
        let policy = Policy::compile(config(POLICY)).unwrap();
        assert_eq!(
            policy.assignments["root@example.com"][0].scopes,
            ["alpha", "beta"]
        );
    }

    #[test]
    fn scopes_are_deduplicated_and_email_keys_lowercased() {
        let policy = Policy::compile(config(POLICY)).unwrap();
        assert_eq!(
            policy.assignments["jane@example.com"][0].scopes,
            ["alpha", "beta"]
        );
        assert!(!policy.assignments.contains_key("Jane@Example.com"));
    }

    #[test]
    fn empty_scope_list_compiles_to_no_scopes() {
        let policy = Policy::compile(config(POLICY)).unwrap();
        assert!(policy.assignments["ci"][0].scopes.is_empty());
    }

    #[test]
    fn roles_resolve_to_permission_sets() {
        let policy = Policy::compile(config(POLICY)).unwrap();
        assert_eq!(policy.roles["admin"].permissions, PermSet::all());
        assert_eq!(policy.roles["admin"].listed, ["*"]);
        let dev = &policy.roles["dev"];
        assert_eq!(
            dev.permissions,
            [Permission::View, Permission::Manage, Permission::Create]
                .into_iter()
                .collect()
        );
        assert_eq!(dev.listed, ["view", "manage", "create"]);
        let publisher = &policy.roles["publisher"];
        assert!(publisher.permissions.is_empty());
        assert_eq!(
            publisher.blueprints,
            BTreeSet::from(["static-html".to_string()])
        );
    }

    #[test]
    fn app_scopes_are_not_part_of_the_policy() {
        let mut config = config(POLICY);
        config.apps.insert("app".into(), vec!["alpha".into()]);
        assert!(Policy::compile(config).unwrap().config().apps.is_empty());
    }

    #[test]
    fn undefined_role_names_user_and_role() {
        let err = compile_err(&POLICY.replace("role: dev", "role: devloper"));
        assert_eq!(
            err,
            PolicyError::UndefinedRole {
                user: "Jane@Example.com".into(),
                role: "devloper".into()
            }
        );
        let message = err.to_string();
        assert!(message.contains("Jane@Example.com") && message.contains("devloper"));
    }

    #[test]
    fn undefined_scope_names_user_and_scope() {
        let err = compile_err(&POLICY.replace("[beta, alpha, beta]", "[beta, stagin]"));
        let message = err.to_string();
        assert!(matches!(err, PolicyError::UndefinedScope { .. }), "{err:?}");
        assert!(
            message.contains("Jane@Example.com") && message.contains("stagin"),
            "{message}"
        );
    }

    #[test]
    fn invalid_permission_built_in_code_names_role_and_permission() {
        let mut config = config(POLICY);
        config.roles.get_mut("publisher").unwrap().permissions =
            vec![PermissionOrWildcard::CreateFromBlueprint("a b".into())];
        let err = Policy::compile(config).unwrap_err();
        assert_eq!(
            err.to_string(),
            "role 'publisher' has invalid permission 'create:a b'"
        );
    }

    #[test]
    fn duplicate_errors_name_the_key() {
        assert!(PolicyError::DuplicateScope("qa".into())
            .to_string()
            .contains("'qa'"));
        assert!(PolicyError::DuplicateRole("auditor".into())
            .to_string()
            .contains("'auditor'"));
    }

    #[test]
    fn perm_set_operations() {
        let mut set = PermSet::default();
        assert!(set.is_empty());
        set.insert(Permission::Logs);
        assert!(set.contains(Permission::Logs));
        assert!(!set.contains(Permission::View));
        let both = set.union([Permission::View].into_iter().collect());
        assert!(both.contains(Permission::View) && both.contains(Permission::Logs));
        assert!(Permission::all()
            .into_iter()
            .all(|p| PermSet::all().contains(p)));
    }
}

#[cfg(test)]
mod grants_tests {
    use super::tests::config;
    use super::*;

    const POLICY: &str = r#"
scopes:
  alpha: { description: Alpha, created_at: 2024-01-01T00:00:00Z }
  beta: { description: Beta, created_at: 2024-01-01T00:00:00Z }
  gamma: { description: Gamma, created_at: 2024-01-01T00:00:00Z }
roles:
  admin: { permissions: ['*'], description: all }
  dev: { permissions: [view, manage, create], description: list }
  viewer: { permissions: [view], description: view }
  publisher: { permissions: ['create:static-html'], description: blueprint }
  nothing: { permissions: [], description: empty }
assignments:
  carol@example.com:
    - { role: viewer, scopes: [alpha] }
  '@example.com':
    - { role: dev, scopes: [beta] }
  identifier:ci:
    - { role: publisher, scopes: [alpha] }
  ghost@other.test:
    - { role: admin, scopes: [] }
  empty@other.test:
    - { role: nothing, scopes: [alpha] }
  '*':
    - { role: viewer, scopes: [gamma] }
"#;

    fn policy() -> Policy {
        Policy::compile(config(POLICY)).unwrap()
    }

    /// Scope -> listed permissions of `user`.
    fn listing(user: &str) -> Vec<(String, Vec<String>)> {
        policy()
            .grants_for(user)
            .scopes()
            .iter()
            .map(|(scope, grant)| (scope.clone(), grant.listed.clone()))
            .collect()
    }

    fn row(scope: &str, listed: &[&str]) -> (String, Vec<String>) {
        (
            scope.to_string(),
            listed.iter().map(|p| p.to_string()).collect(),
        )
    }

    #[test]
    fn exact_entry_matches_in_any_case_and_adds_domain_and_wildcard() {
        let expected = vec![
            row("alpha", &["view"]),
            row("beta", &["view", "manage", "create"]),
            row("gamma", &["view"]),
        ];
        assert_eq!(listing("carol@example.com"), expected);
        assert_eq!(listing("Carol@Example.COM"), expected);
    }

    #[test]
    fn domain_entry_matches_its_domain_in_any_case() {
        let expected = vec![
            row("beta", &["view", "manage", "create"]),
            row("gamma", &["view"]),
        ];
        assert_eq!(listing("erin@example.com"), expected);
        assert_eq!(listing("Erin@EXAMPLE.com"), expected);
    }

    #[test]
    fn domain_entry_does_not_match_a_subdomain() {
        assert_eq!(
            listing("frank@sub.example.com"),
            vec![row("gamma", &["view"])]
        );
    }

    #[test]
    fn identifier_matches_case_sensitively() {
        assert_eq!(
            listing("identifier:ci"),
            vec![
                row("alpha", &["create:static-html"]),
                row("gamma", &["view"])
            ]
        );
        assert_eq!(listing("identifier:CI"), vec![row("gamma", &["view"])]);
    }

    #[test]
    fn wildcard_matches_everyone() {
        assert_eq!(listing("zed@nowhere.test"), vec![row("gamma", &["view"])]);
        assert_eq!(listing("not-an-email"), vec![row("gamma", &["view"])]);
    }

    #[test]
    fn exact_and_domain_permissions_add_up_in_a_shared_scope() {
        let grants = policy().grants_for("carol@example.com");
        let beta = &grants.scopes()["beta"];
        assert!(beta.permissions.contains(Permission::Manage));
        assert!(grants.scopes()["alpha"]
            .permissions
            .contains(Permission::View));
        assert!(!grants.scopes()["alpha"]
            .permissions
            .contains(Permission::Manage));
    }

    #[test]
    fn assignment_without_scopes_grants_nothing() {
        assert_eq!(listing("ghost@other.test"), vec![row("gamma", &["view"])]);
    }

    #[test]
    fn blueprint_grants_are_kept_per_scope() {
        let grants = policy().grants_for("identifier:ci");
        let alpha = &grants.scopes()["alpha"];
        assert!(alpha.permissions.is_empty());
        assert!(alpha.blueprints.contains("static-html"));
    }

    #[test]
    fn grants_are_empty_without_any_permission() {
        let mut config = config(POLICY);
        config.assignments.remove("*");
        let policy = Policy::compile(config).unwrap();
        assert!(policy.grants_for("nobody@other.test").is_empty());
        // A role without permissions is listed but grants nothing.
        let empty = policy.grants_for("empty@other.test");
        assert_eq!(empty.scopes().len(), 1);
        assert!(empty.is_empty());
        assert!(!policy.grants_for("identifier:ci").is_empty());
    }
}
