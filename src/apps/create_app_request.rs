use super::{app_data::AppSettings, file_list::FileList};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CreateAppRequest {
    pub app_name: String,
    pub settings: AppSettings,
    pub files: FileList,
}
