use std::collections::HashMap;

use serde::Deserialize;

use super::app_blueprint::AppBlueprintMap;
use super::scheduler_interval::SchedulerInterval;

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
pub struct Apps {
    pub root_folder: String,
    pub max_depth: u32,
    pub domain_suffix: String,
    pub blueprints: AppBlueprintMap,
    /// Per-deployment domain suffix per blueprint name; wins over the
    /// blueprint's own `domain_suffix` (directory blueprints bypass `config`).
    #[serde(default)]
    pub blueprint_domain_suffixes: HashMap<String, String>,
    /// An operation running longer than this is aborted and its app unlocked.
    #[serde(default = "default_max_operation_duration")]
    pub max_operation_duration: SchedulerInterval,
    /// Path prefixes the load balancer refuses for every app.
    #[serde(default)]
    pub blocked_paths: Vec<String>,
}

fn default_max_operation_duration() -> SchedulerInterval {
    SchedulerInterval::Hours(1)
}

impl Apps {
    /// Domain suffix for an app: config override for its blueprint, then the
    /// blueprint's own suffix, then the global one.
    pub fn domain_suffix_for(&self, blueprint: Option<&str>) -> &str {
        blueprint
            .and_then(|name| {
                self.blueprint_domain_suffixes.get(name).or_else(|| {
                    self.blueprints
                        .get(name)?
                        .template
                        .as_ref()?
                        .domain_suffix
                        .as_ref()
                })
            })
            .map_or(&self.domain_suffix, |s| s)
    }

    /// Effective blocked paths of an app: the global list plus the list of
    /// its blueprint, deduplicated and sorted. Read from the current config,
    /// so an unknown blueprint contributes nothing.
    pub fn blocked_paths_for(&self, blueprint: Option<&str>) -> Vec<String> {
        let from_blueprint = blueprint
            .and_then(|name| self.blueprints.get(name))
            .map(|bp| bp.blocked_paths.as_slice())
            .unwrap_or_default();
        let set: std::collections::BTreeSet<&String> =
            self.blocked_paths.iter().chain(from_blueprint).collect();
        set.into_iter().cloned().collect()
    }

    /// Every blocked path entry ends up in a Traefik rule and a Compose
    /// label, so only a safe path character set is allowed (no backtick,
    /// no `$`), and `/` alone would block the whole app.
    pub fn check_blocked_paths(&self) -> Result<(), String> {
        let origins = std::iter::once(("the global list".to_string(), &self.blocked_paths)).chain(
            self.blueprints
                .iter()
                .map(|(name, bp)| (format!("blueprint {name}"), &bp.blocked_paths)),
        );
        for (origin, entries) in origins {
            for entry in entries {
                let valid = entry.len() > 1
                    && entry.starts_with('/')
                    && entry
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "/._~%-".contains(c));
                if !valid {
                    return Err(format!(
                        "invalid blocked path {entry:?} in {origin}: must start with '/', not be '/' alone and contain only letters, digits and / . _ ~ - %"
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Default for Apps {
    fn default() -> Self {
        Apps {
            root_folder: ".".to_string(),
            max_depth: 3,
            domain_suffix: "".to_string(),
            blueprints: HashMap::new(),
            blueprint_domain_suffixes: HashMap::new(),
            max_operation_duration: default_max_operation_duration(),
            blocked_paths: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::app_blueprint::AppBlueprint;

    fn apps(global: &[&str], bp: &[&str]) -> Apps {
        let blueprint = AppBlueprint {
            blocked_paths: bp.iter().map(|s| s.to_string()).collect(),
            ..serde_norway::from_str(
                "{name: d, description: d, actions: {}, required_services: []}",
            )
            .unwrap()
        };
        Apps {
            blocked_paths: global.iter().map(|s| s.to_string()).collect(),
            blueprints: HashMap::from([("drupal".to_string(), blueprint)]),
            ..Default::default()
        }
    }

    #[test]
    fn blocked_paths_union_dedup_sorted() {
        let a = apps(
            &["/core/install.php", "/.git/"],
            &["/core/rebuild.php", "/core/install.php"],
        );
        assert_eq!(
            a.blocked_paths_for(Some("drupal")),
            vec!["/.git/", "/core/install.php", "/core/rebuild.php"]
        );
    }

    #[test]
    fn blocked_paths_global_only_without_or_unknown_blueprint() {
        let a = apps(&["/.git/"], &["/core/install.php"]);
        assert_eq!(a.blocked_paths_for(None), vec!["/.git/"]);
        assert_eq!(a.blocked_paths_for(Some("gone")), vec!["/.git/"]);
    }

    #[test]
    fn blocked_paths_validation() {
        assert!(apps(&["/.git/"], &["/core/install.php"])
            .check_blocked_paths()
            .is_ok());
        let err = apps(&[], &["/"]).check_blocked_paths().unwrap_err();
        assert!(
            err.contains("\"/\"") && err.contains("blueprint drupal"),
            "{err}"
        );
        let err = apps(&["/core/`x`"], &[]).check_blocked_paths().unwrap_err();
        assert!(err.contains("global"), "{err}");
        assert!(apps(&["/a$b"], &[]).check_blocked_paths().is_err());
        assert!(apps(&["core"], &[]).check_blocked_paths().is_err());
    }
}
