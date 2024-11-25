#![allow(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct NotificationServiceSettings {
    services: HashMap<String, NotificationServiceType>,
}

impl<'de> Deserialize<'de> for NotificationServiceSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let map: HashMap<String, serde_json::Value> = Deserialize::deserialize(deserializer)?;
        let mut services: HashMap<String, NotificationServiceType> = HashMap::new();

        for (key, mut value) in map {
            let service_type = value
                .as_object_mut()
                .and_then(|obj| obj.remove("type"))
                .ok_or_else(|| serde::de::Error::missing_field("type"))?;
            let service = match service_type.as_str() {
                Some("mattermost") => match MattermostSettings::deserialize(value) {
                    Ok(settings) => NotificationServiceType::Mattermost(settings),
                    Err(e) => return Err(serde::de::Error::custom(e.to_string())),
                },
                Some("gitlab") => match GitlabSettings::deserialize(value) {
                    Ok(settings) => NotificationServiceType::Gitlab(settings),
                    Err(e) => return Err(serde::de::Error::custom(e.to_string())),
                },
                _ => {
                    return Err(serde::de::Error::custom(format!(
                        "Unknown service type: {}",
                        service_type
                    )))
                }
            };
            services.insert(key, service);
        }

        Ok(NotificationServiceSettings { services })
    }
}

impl NotificationServiceSettings {
    pub fn get_mattermost(&self, service_id: &str) -> Option<&MattermostSettings> {
        match self.services.get(service_id) {
            Some(NotificationServiceType::Mattermost(settings)) => Some(settings),
            _ => None,
        }
    }

    pub fn get_gitlab(&self, service_is: &str) -> Option<&GitlabSettings> {
        match self.services.get(service_is) {
            Some(NotificationServiceType::Gitlab(settings)) => Some(settings),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[readonly::make]
pub struct MattermostSettings {
    pub host: String,
    pub hook_id: String,
}

#[derive(Debug, Deserialize, Clone)]
#[readonly::make]
pub struct GitlabSettings {
    pub host: String,
    pub token: String,
}

#[derive(Debug, Deserialize, Clone)]
pub enum NotificationServiceType {
    Mattermost(MattermostSettings),
    Gitlab(GitlabSettings),
}
