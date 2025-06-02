use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
        match self {
            ActionName::PostCreate => serializer.serialize_str("post_create"),
            ActionName::PostRun => serializer.serialize_str("post_run"),
            ActionName::PostRebuild => serializer.serialize_str("post_rebuild"),
            ActionName::Custom(name) => serializer.serialize_str(name),
        }
    }
}

struct ActionNameVisitor;

impl serde::de::Visitor<'_> for ActionNameVisitor {
    type Value = ActionName;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string representing an action name")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match value {
            "post_create" => Ok(ActionName::PostCreate),
            "post_run" => Ok(ActionName::PostRun),
            "post_rebuild" => Ok(ActionName::PostRebuild),
            custom => Ok(ActionName::Custom(custom.to_string())),
        }
    }
}

impl<'de> Deserialize<'de> for ActionName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ActionNameVisitor)
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
