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

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema, utoipa::ToResponse)]
#[allow(unused)]
#[serde(try_from = "AppBlueprintShadow")]
pub struct AppBlueprint {
    pub name: String,
    pub description: String,
    pub actions: HashMap<ActionName, HashMap<String, Vec<String>>>,
    pub required_services: Vec<String>,
    pub public_services: Option<HashMap<String, u16>>,
}

#[derive(Deserialize)]
pub struct AppBlueprintShadow {
    pub name: String,
    pub description: String,
    pub actions: HashMap<ActionName, HashMap<String, Vec<String>>>,
    pub required_services: Vec<String>,
    pub public_services: Option<HashMap<String, u16>>,
}

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
impl std::convert::TryFrom<AppBlueprintShadow> for AppBlueprint {
    type Error = AppBlueprintValidationError;
    fn try_from(shadow: AppBlueprintShadow) -> Result<Self, Self::Error> {
        for public_service in shadow
            .public_services
            .as_ref()
            .unwrap_or(&HashMap::new())
            .keys()
        {
            if !shadow.required_services.contains(public_service) {
                return Err(AppBlueprintValidationError {
                    msg: format!(
                        "Public service {} not in required services",
                        &public_service
                    ),
                });
            }
        }
        for (action, services) in &shadow.actions {
            for service in services.keys() {
                if !shadow.required_services.contains(service) {
                    return Err(AppBlueprintValidationError {
                        msg: format!(
                            "service {} required for action {:?} not in required services",
                            &service, &action
                        ),
                    });
                }
            }
        }
        Ok(AppBlueprint {
            name: shadow.name,
            description: shadow.description,
            actions: shadow.actions,
            required_services: shadow.required_services,
            public_services: shadow.public_services,
        })
    }
}

pub type AppBlueprintMap = HashMap<String, AppBlueprint>;
