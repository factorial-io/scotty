use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize, Clone)]
#[allow(unused)]
#[readonly::make]
pub struct ApiServer {
    pub bind_address: String,
    pub access_token: Option<String>,
    #[serde(deserialize_with = "deserialize_bytes")]
    pub create_app_max_size: usize,
}

impl Default for ApiServer {
    fn default() -> Self {
        ApiServer {
            bind_address: "0.0.0.0:21342".to_string(),
            access_token: None,
            create_app_max_size: 1024 * 1024 * 10,
        }
    }
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let s = s.trim().to_uppercase();

    let (num_part, suffix) = s.split_at(s.len().saturating_sub(1));
    let multiplier = match suffix {
        "G" => 1_024 * 1_024 * 1_024,
        "M" => 1_024 * 1_024,
        "K" => 1_024,
        _ => 1,
    };

    let num: usize = num_part.parse().map_err(serde::de::Error::custom)?;
    Ok(num * multiplier)
}
