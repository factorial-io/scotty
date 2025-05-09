use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct File {
    pub name: String,
    #[schema(value_type = String, format = "Base64")]
    pub content: Vec<u8>,
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("name", &self.name)
            .field("content", &format!("<{} bytes>", self.content.len()))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FileList {
    pub files: Vec<File>,
}
