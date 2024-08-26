use super::{app_data::AppSettings, file_list::FileList};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CreateAppRequest {
    #[serde(
        serialize_with = "serialize_app_name",
        deserialize_with = "deserialize_app_name"
    )]
    pub app_name: String,
    pub settings: AppSettings,
    pub files: FileList,
}

fn serialize_app_name<S>(app_name: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let slugified_name = slug::slugify(app_name);
    serializer.serialize_str(&slugified_name)
}

fn deserialize_app_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let app_name: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(slug::slugify(app_name))
}
