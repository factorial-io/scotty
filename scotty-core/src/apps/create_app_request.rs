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
}
