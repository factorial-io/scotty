use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct File {
    pub name: String,
    #[schema(value_type = String, format = "Base64")]
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FileList {
    pub files: Vec<File>,
}
