//! Application context

use std::sync::Arc;

use emily_client::apis::configuration::Configuration as EmilyConfig;

use crate::bitcoin::node::BitcoinCoreClient;
use crate::config::Settings;
use crate::error::Error;

/// Application context
#[derive(Clone)]
pub struct Context {
    bitcoin_client: BitcoinCoreClient,
    emily_config: Arc<EmilyConfig>,
}

impl TryFrom<&Settings> for Context {
    type Error = Error;

    fn try_from(value: &Settings) -> Result<Self, Self::Error> {
        let bitcoin_client = BitcoinCoreClient::try_from(&value.bitcoin_rpc_endpoint)?;
        let emily_config = EmilyConfig {
            base_path: value
                .emily_endpoint
                .to_string()
                .trim_end_matches("/")
                .to_string(),
            ..Default::default()
        };

        Ok(Self {
            bitcoin_client,
            emily_config: Arc::new(emily_config),
        })
    }
}

impl Context {
    /// Get a reference to the Bitcoin client
    pub fn bitcoin_client(&self) -> &BitcoinCoreClient {
        &self.bitcoin_client
    }

    /// Get a reference to the Emily config
    pub fn emily_config(&self) -> &EmilyConfig {
        &self.emily_config
    }
}
