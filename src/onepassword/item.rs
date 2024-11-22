#[allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub vault: Vault,
    pub category: String,
    #[serde(default)]
    pub sections: Vec<Section>,
    pub fields: Vec<Field>,
    #[serde(default)]
    pub files: Vec<File>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Vault {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub purpose: Option<String>,
    pub label: String,
    pub value: Option<String>,
    pub entropy: Option<f64>,
    pub section: Option<FieldSection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FieldSection {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub name: String,
    pub size: u64,
    #[serde(rename = "content_path")]
    pub content_path: String,
}

impl Item {
    pub fn has_field(&self, field_id: &str) -> bool {
        self.fields.iter().any(|field| field.id == field_id)
    }

    /// Get a field by its ID or label.
    pub fn get_field(&self, field_id: &str, section_label: Option<&str>) -> Option<&Field> {
        match section_label {
            Some(section_label) => {
                let section_id = &self
                    .sections
                    .iter()
                    .find(|section| section.label == section_label)?
                    .id;
                self.fields.iter().find(|field| {
                    (field.id == field_id || field.label == field_id)
                        && field.section.as_ref().map(|section| section.id.as_str())
                            == Some(section_id.as_str())
                })
            }
            None => self
                .fields
                .iter()
                .find(|field| (field.id == field_id || field.label == field_id)),
        }
    }

    pub fn get_field_value(&self, field_id: &str, section_label: Option<&str>) -> Option<&str> {
        self.get_field(field_id, section_label)
            .and_then(|field| field.value.as_deref())
    }

    pub fn get_password(&self) -> Option<&str> {
        self.fields
            .iter()
            .find(|field| {
                field.id == "password"
                    || field.field_type == "password"
                    || field.field_type == "CONCEALED"
            })
            .and_then(|field| field.value.as_deref())
    }
}
