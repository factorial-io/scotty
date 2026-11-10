use super::{app_data::AppSettings, file_list::FileList};
use crate::utils::serde::{deserialize_app_name, serialize_app_name};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CustomDomainMapping {
    pub domain: String,
    pub service: String,
}
fn default_scopes() -> Vec<String> {
    vec!["default".to_string()]
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CreateAppRequest {
    #[serde(
        serialize_with = "serialize_app_name",
        deserialize_with = "deserialize_app_name"
    )]
    pub app_name: String,
    pub settings: AppSettings,
    pub files: FileList,
    pub custom_domains: Vec<CustomDomainMapping>,
    #[serde(default = "default_scopes")]
    pub requested_scopes: Vec<String>,
    /// Parameters for a templated blueprint, validated against its schema.
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    #[schema(value_type = Object)]
    pub blueprint_parameters: serde_json::Map<String, serde_json::Value>,
    /// One of the templated blueprint's allowed images; defaults to the first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_request_json_still_deserializes() {
        let req: CreateAppRequest = serde_json::from_value(serde_json::json!({
            "app_name": "demo",
            "settings": serde_json::to_value(AppSettings::default()).unwrap(),
            "files": {"files": []},
            "custom_domains": [],
        }))
        .unwrap();
        assert!(req.blueprint_parameters.is_empty());
        assert!(req.image.is_none());
    }

    #[test]
    fn template_fields_round_trip() {
        let mut req: CreateAppRequest = serde_json::from_value(serde_json::json!({
            "app_name": "demo",
            "settings": serde_json::to_value(AppSettings::default()).unwrap(),
            "files": {"files": []},
            "custom_domains": [],
        }))
        .unwrap();
        req.blueprint_parameters
            .insert("SITE_NAME".into(), serde_json::json!("Demo"));
        req.image = Some("img:1".into());
        let back: CreateAppRequest =
            serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert_eq!(back.blueprint_parameters["SITE_NAME"], "Demo");
        assert_eq!(back.image.as_deref(), Some("img:1"));
    }
}
