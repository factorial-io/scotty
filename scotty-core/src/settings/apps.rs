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
}

impl Default for Apps {
    fn default() -> Self {
        Apps {
            root_folder: ".".to_string(),
            max_depth: 3,
            domain_suffix: "".to_string(),
            blueprints: HashMap::new(),
        }
    }
}
