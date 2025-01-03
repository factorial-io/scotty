pub fn serialize_app_name<S>(app_name: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let slugified_name = slug::slugify(app_name);
    serializer.serialize_str(&slugified_name)
}

pub fn deserialize_app_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let app_name: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(slug::slugify(app_name))
}
