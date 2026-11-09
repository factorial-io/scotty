use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::authorization::Permission;

#[derive(Clone, Debug, Hash, Eq, PartialEq, utoipa::ToSchema, utoipa::ToResponse)]
pub enum ActionType {
    Lifecycle,
    Custom,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Lifecycle => "lifecycle",
            ActionType::Custom => "custom",
        }
    }
}

impl From<ActionType> for String {
    fn from(action_type: ActionType) -> Self {
        action_type.as_str().to_string()
    }
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, utoipa::ToSchema, utoipa::ToResponse)]
#[allow(clippy::enum_variant_names)]
pub enum ActionName {
    PostCreate,
    PostRun,
    PostRebuild,
    Custom(String),
}

impl Serialize for ActionName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s: String = self.clone().into();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for ActionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl From<ActionName> for String {
    fn from(action_name: ActionName) -> Self {
        match action_name {
            ActionName::PostCreate => "post_create".to_string(),
            ActionName::PostRun => "post_run".to_string(),
            ActionName::PostRebuild => "post_rebuild".to_string(),
            ActionName::Custom(name) => name,
        }
    }
}

impl From<String> for ActionName {
    fn from(value: String) -> Self {
        match value.as_str() {
            "post_create" => ActionName::PostCreate,
            "post_run" => ActionName::PostRun,
            "post_rebuild" => ActionName::PostRebuild,
            custom => ActionName::Custom(custom.to_string()),
        }
    }
}

impl ActionName {
    pub fn get_type(&self) -> ActionType {
        match self {
            ActionName::PostCreate => ActionType::Lifecycle,
            ActionName::PostRun => ActionType::Lifecycle,
            ActionName::PostRebuild => ActionType::Lifecycle,
            ActionName::Custom(_) => ActionType::Custom,
        }
    }
}

// ServiceCommands struct no longer needed since we're using a HashMap directly

/// A blueprint action that can be executed on apps using this blueprint.
///
/// Blueprint actions use the same permission model as per-app custom actions:
/// - `action_read` permission allows executing read-only actions
/// - `action_write` permission allows executing actions that modify state
///
/// The key difference is that blueprint actions are always executable (no approval
/// workflow), while per-app custom actions require approval before they can be run.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct Action {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub commands: HashMap<String, Vec<String>>,
    /// Required permission to execute this action (defaults to ActionWrite).
    /// Both blueprint actions and per-app custom actions use the same permission types.
    #[serde(default = "Action::default_permission")]
    pub permission: Permission,
}

impl Action {
    pub fn get_commands_for_service(&self, service: &str) -> Option<&Vec<String>> {
        self.commands.get(service)
    }

    pub fn new(description: String, commands: HashMap<String, Vec<String>>) -> Self {
        Self {
            description,
            commands,
            permission: Self::default_permission(),
        }
    }

    pub fn with_permission(
        description: String,
        commands: HashMap<String, Vec<String>>,
        permission: Permission,
    ) -> Self {
        Self {
            description,
            commands,
            permission,
        }
    }

    /// Default permission for actions (ActionWrite for backwards compatibility)
    fn default_permission() -> Permission {
        Permission::ActionWrite
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AppBlueprint {
    pub name: String,
    pub description: String,
    pub actions: HashMap<ActionName, Action>,
    pub required_services: Vec<String>,
    pub public_services: Option<HashMap<String, u16>>,
    /// Set for directory blueprints that ship their own compose file.
    /// `None` means a validation-only blueprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<BlueprintTemplate>,
    /// True for directory blueprints. Set by the loader, never read from YAML.
    #[serde(default, skip_deserializing)]
    pub templated: bool,
}

/// The parts of a templated blueprint (`config/blueprints/<name>/`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BlueprintTemplate {
    /// Every file of the blueprint directory except `blueprint.yaml`,
    /// with paths relative to it. Loaded from disk, never (de)serialized.
    #[serde(skip)]
    #[schema(ignore)]
    pub files: Vec<crate::apps::file_list::File>,
    /// JSON Schema (`type: object`) for the caller's parameters.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub parameters: Option<serde_json::Value>,
    /// Allowed images; the first is the default. Passed to compose as `IMAGE`.
    #[serde(default)]
    pub images: Vec<String>,
    /// Glob patterns for caller files. Empty means no caller files.
    #[serde(default)]
    pub payload: Vec<String>,
    /// Overrides `apps.domain_suffix` for apps from this blueprint.
    #[serde(default)]
    pub domain_suffix: Option<String>,
}

#[derive(Debug)]
pub struct AppBlueprintValidationError {
    msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AppBlueprintList {
    pub blueprints: HashMap<String, AppBlueprint>,
}

// The error type has to implement Display
impl std::fmt::Display for AppBlueprintValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "AppBlueprint didnt validate: {}", self.msg)
    }
}

impl std::error::Error for AppBlueprintValidationError {}

impl AppBlueprint {
    pub fn get_action(&self, action_name: &ActionName) -> Option<&Action> {
        self.actions.get(action_name)
    }

    pub fn get_commands_for_service(
        &self,
        action_name: &ActionName,
        service: &str,
    ) -> Option<&Vec<String>> {
        self.get_action(action_name)
            .and_then(|action| action.get_commands_for_service(service))
    }

    pub fn get_services_for_action(&self, action_name: &ActionName) -> Option<Vec<&str>> {
        self.get_action(action_name)
            .map(|action| action.commands.keys().map(|s| s.as_str()).collect())
    }

    pub fn validate(&self) -> Result<(), AppBlueprintValidationError> {
        // Validate that all public services are in the required services list
        for public_service in self
            .public_services
            .as_ref()
            .unwrap_or(&HashMap::new())
            .keys()
        {
            if !self.required_services.contains(public_service) {
                return Err(AppBlueprintValidationError {
                    msg: format!("Public service {} not in required services", public_service),
                });
            }
        }

        // Validate that all services used in actions are in the required services list
        for (action_name, action) in &self.actions {
            for service in action.commands.keys() {
                if !self.required_services.contains(service) {
                    return Err(AppBlueprintValidationError {
                        msg: format!(
                            "service {} required for action {:?} not in required services",
                            service, action_name
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

pub type AppBlueprintMap = HashMap<String, AppBlueprint>;

/// Loads templated blueprints from `<dir>/<name>/blueprint.yaml` and inserts
/// them into `blueprints`, keyed by directory name. Fails on a missing compose
/// file, unparsable YAML, a non-flat parameter schema or a name clash.
pub fn load_blueprint_dirs(
    dir: &std::path::Path,
    blueprints: &mut AppBlueprintMap,
) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let blueprint = load_blueprint_dir(&path).map_err(|e| format!("blueprint {name}: {e}"))?;
        if blueprints.contains_key(&name) {
            return Err(format!(
                "blueprint {name}: defined both as YAML file and as directory"
            ));
        }
        blueprints.insert(name, blueprint);
    }
    Ok(())
}

fn load_blueprint_dir(path: &std::path::Path) -> Result<AppBlueprint, String> {
    let yaml = std::fs::read_to_string(path.join("blueprint.yaml"))
        .map_err(|e| format!("cannot read blueprint.yaml: {e}"))?;
    let mut blueprint: AppBlueprint =
        serde_norway::from_str(&yaml).map_err(|e| format!("invalid blueprint.yaml: {e}"))?;
    if crate::utils::compose::find_config_file_in_dir(path).is_none() {
        return Err("no compose file".into());
    }

    let mut files = vec![];
    for entry in walkdir::WalkDir::new(path).sort_by_file_name() {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(path)
            .expect("walkdir stays below root");
        if rel == std::path::Path::new("blueprint.yaml") {
            continue;
        }
        let bytes = std::fs::read(entry.path()).map_err(|e| format!("{}: {e}", rel.display()))?;
        files.push(crate::apps::file_list::File {
            name: rel.to_string_lossy().to_string(),
            content: crate::apps::file_list::FileContent::new(bytes),
            compressed: false,
        });
    }

    let mut template = blueprint.template.take().unwrap_or_default();
    template.files = files;
    if let Some(schema) = &template.parameters {
        check_flat_schema(schema)?;
    }
    blueprint.template = Some(template);
    blueprint.templated = true;
    blueprint.validate().map_err(|e| e.to_string())?;
    Ok(blueprint)
}

/// Parameters become env vars, so only flat scalar properties are allowed.
fn check_flat_schema(schema: &serde_json::Value) -> Result<(), String> {
    if schema.get("type").and_then(|t| t.as_str()) != Some("object") {
        return Err("parameters must be a JSON Schema with type: object".into());
    }
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return Ok(());
    };
    for (key, prop) in props {
        match prop.get("type").and_then(|t| t.as_str()) {
            Some("string" | "number" | "integer" | "boolean") => {}
            _ => {
                return Err(format!(
                    "parameter {key} must have type string, number, integer or boolean"
                ))
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod blueprint_dir_tests {
    use super::*;
    use std::fs;

    fn write(dir: &std::path::Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    const YAML: &str = "name: Demo\ndescription: d\nactions: {}\nrequired_services: [web]\npublic_services: {web: 80}\n";

    fn load(setup: impl Fn(&std::path::Path)) -> Result<AppBlueprintMap, String> {
        let root = tempfile::tempdir().unwrap();
        let bp = root.path().join("demo");
        fs::create_dir(&bp).unwrap();
        setup(&bp);
        let mut map = AppBlueprintMap::new();
        load_blueprint_dirs(root.path(), &mut map).map(|_| map)
    }

    #[test]
    fn valid_directory() {
        let map = load(|d| {
            write(d, "blueprint.yaml", &format!("{YAML}template:\n  images: [a, b]\n  payload: ['html/**']\n  parameters: {{type: object, properties: {{X: {{type: string}}}}}}\n"));
            write(d, "compose.yml", "services: {}");
            write(d, "conf/sws.toml", "x");
        })
        .unwrap();
        let t = map["demo"].template.as_ref().unwrap();
        let mut names: Vec<_> = t.files.iter().map(|f| f.name.as_str()).collect();
        names.sort();
        assert_eq!(names, ["compose.yml", "conf/sws.toml"]);
        assert_eq!(t.images, ["a", "b"]);
    }

    #[test]
    fn template_key_is_optional() {
        let map = load(|d| {
            write(d, "blueprint.yaml", YAML);
            write(d, "docker-compose.yml", "services: {}");
        })
        .unwrap();
        assert!(map["demo"].template.is_some());
    }

    #[test]
    fn missing_compose_fails() {
        let err = load(|d| write(d, "blueprint.yaml", YAML)).unwrap_err();
        assert!(err.contains("demo") && err.contains("compose"), "{err}");
    }

    #[test]
    fn unparsable_yaml_fails() {
        let err = load(|d| {
            write(d, "blueprint.yaml", "name: [");
            write(d, "compose.yml", "services: {}");
        })
        .unwrap_err();
        assert!(err.contains("demo"), "{err}");
    }

    #[test]
    fn nested_schema_fails() {
        let err = load(|d| {
            write(d, "blueprint.yaml", &format!("{YAML}template:\n  parameters: {{type: object, properties: {{X: {{type: object}}}}}}\n"));
            write(d, "compose.yml", "services: {}");
        })
        .unwrap_err();
        assert!(err.contains("X"), "{err}");
    }

    #[test]
    fn name_clash_fails() {
        let root = tempfile::tempdir().unwrap();
        write(&root.path().join("demo"), "blueprint.yaml", YAML);
        write(&root.path().join("demo"), "compose.yml", "services: {}");
        let mut map = AppBlueprintMap::new();
        map.insert("demo".into(), serde_norway::from_str(YAML).unwrap());
        let err = load_blueprint_dirs(root.path(), &mut map).unwrap_err();
        assert!(err.contains("both"), "{err}");
    }
}
