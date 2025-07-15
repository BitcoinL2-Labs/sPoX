use serde::{Deserialize, Deserializer};

/// A deserializer for the url::Url type. Does not support deserializing a list,
/// only a single URL.
pub fn url_deserializer<'de, D>(deserializer: D) -> Result<url::Url, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(serde::de::Error::custom)
}

/// A deserializer for the std::time::Duration type.
/// Serde includes a default deserializer, but it expects a struct.
pub fn duration_seconds_deserializer<'de, D>(
    deserializer: D,
) -> Result<std::time::Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(std::time::Duration::from_secs(
        u64::deserialize(deserializer).map_err(serde::de::Error::custom)?,
    ))
}
