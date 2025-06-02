use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct Action {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    pub commands: HashMap<String, Vec<String>>,
}

impl Action {
    pub fn get_commands_for_service(&self, service: &str) -> Option<&Vec<String>> {
        self.commands.get(service)
    }

    pub fn new(description: String, commands: HashMap<String, Vec<String>>) -> Self {
        Self {
            description,
            commands,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::ToResponse)]
pub struct AppBlueprint {
    pub name: String,
    pub description: String,
    pub actions: HashMap<ActionName, Action>,
    pub required_services: Vec<String>,
    pub public_services: Option<HashMap<String, u16>>,
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
        write!(formatter, "AppBlueprint didnt validate: {}", &self.msg)
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
                    msg: format!(
                        "Public service {} not in required services",
                        &public_service
                    ),
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
                            &service, &action_name
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

pub type AppBlueprintMap = HashMap<String, AppBlueprint>;
