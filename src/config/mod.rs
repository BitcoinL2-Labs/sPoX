//! sPoX Configuration
use std::path::Path;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use url::Url;

use crate::config::error::SpoxConfigError;
use crate::config::serialization::{duration_seconds_deserializer, url_deserializer_single};

mod error;
mod serialization;

/// Config environment variables prefix
pub const CONFIG_PREFIX: &str = "SPOX";

/// Top-level configuration
#[derive(Deserialize, Clone, Debug)]
pub struct Settings {
    /// Bitcoin RPC endpoint
    #[serde(deserialize_with = "url_deserializer_single")]
    pub bitcoin_rpc_endpoint: Url,
    /// Emily API endpoint
    #[serde(deserialize_with = "url_deserializer_single")]
    pub emily_endpoint: Url,
    /// How often looking for new deposit transactions
    #[serde(deserialize_with = "duration_seconds_deserializer")]
    pub polling_interval: std::time::Duration,
    // TODO: stanze for lookup
}

impl Settings {
    /// Initializing the global config first with default values and then with
    /// provided/overwritten environment variables. The explicit separator with
    /// double underscores is needed to correctly parse the nested config structure.
    ///
    /// The environment variables are prefixed with `SPOX_` and the nested
    /// fields are separated with double underscores.
    pub fn new(config_path: Option<impl AsRef<Path>>) -> Result<Self, ConfigError> {
        // To properly parse lists from both environment and config files while
        // using a custom deserializer, we need to specify the list separator,
        // enable try_parsing and specify the keys which should be parsed as lists.
        // If the keys aren't specified, the deserializer will try to parse all
        // Strings as lists which will result in an error.
        let env = Environment::with_prefix(CONFIG_PREFIX)
            .prefix_separator("_")
            .separator("__")
            .try_parsing(true);

        let mut cfg_builder = Config::builder();

        cfg_builder = cfg_builder.set_default("polling_interval", 30)?;

        if let Some(path) = config_path {
            cfg_builder = cfg_builder.add_source(File::from(path.as_ref()));
        }
        cfg_builder = cfg_builder.add_source(env);

        let cfg = cfg_builder.build()?;

        let settings: Settings = cfg.try_deserialize()?;

        settings.validate()?;

        Ok(settings)
    }

    /// Perform validation on the configuration.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.polling_interval.is_zero() {
            return Err(ConfigError::Message(
                SpoxConfigError::ZeroDurationForbidden("polling_interval").to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use test_case::test_case;

    use super::*;
    use crate::testing::{clear_env, set_var};

    /// Helper function to quickly create a URL from a string in tests.
    fn url(s: &str) -> url::Url {
        s.parse().unwrap()
    }

    /// This test checks that the default configuration values are loaded
    /// correctly from the default.toml file. The Stacks settings are excluded
    /// as they are covered by the [`default_config_toml_loads_with_environment`]
    /// test.
    // !! NOTE: This test needs to be updated if the default values in the
    // !! default.toml file are changed.
    #[test]
    fn default_config_toml_loads() {
        clear_env();

        let settings = Settings::new_from_default_config()
            .expect("Failed create settings from default config");

        assert_eq!(settings.emily_endpoint, url("http://127.0.0.1:3031"));
        assert_eq!(
            settings.bitcoin_rpc_endpoint,
            url("http://devnet:devnet@127.0.0.1:18443")
        );
        assert_eq!(settings.polling_interval, Duration::from_secs(30));
    }

    #[test]
    fn default_config_toml_loads_with_environment() {
        clear_env();

        set_var("SPOX_POLLING_INTERVAL", "31");

        let settings = Settings::new_from_default_config().unwrap();

        assert_eq!(settings.polling_interval, Duration::from_secs(31));
    }

    #[test_case("polling_interval"; "polling interval")]
    fn zero_values_for_nonzero_fields_fail_in_config(field: &str) {
        clear_env();

        set_var(format!("SPOX_{}", field.to_uppercase()), "0");

        Settings::new_from_default_config().expect_err("value for must be non zero");
    }
}
