use std::collections::HashMap;

use serde::Deserialize;

use super::app_blueprint::AppBlueprintMap;

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
}

impl Default for Apps {
    fn default() -> Self {
        Apps {
            root_folder: ".".to_string(),
            max_depth: 3,
            domain_suffix: "".to_string(),
            blueprints: HashMap::new(),
            blueprint_domain_suffixes: HashMap::new(),
        }
    }
}
