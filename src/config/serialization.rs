use clarity::types::chainstate::StacksAddress;
use clarity::vm::types::PrincipalData;
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

/// Parse the string into a StacksAddress.
///
/// The [`StacksAddress`] struct does not implement any string parsing or
/// c32 decoding. However, the [`PrincipalData::parse_standard_principal`]
/// function does the expected c32 decoding and the validation, so we go
/// through that.
pub fn stacks_address_deserializer<'de, D>(des: D) -> Result<StacksAddress, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let literal = <String>::deserialize(des)?;

    PrincipalData::parse_standard_principal(&literal)
        .map(StacksAddress::from)
        .map_err(serde::de::Error::custom)
}
