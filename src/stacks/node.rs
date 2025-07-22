//! A module with structs that interact with the Stacks API.

use std::borrow::Cow;
use std::time::Duration;

use bitcoin::PublicKey;
use clarity::types::chainstate::StacksAddress;
use clarity::vm::types::{BuffData, SequenceData};
use clarity::vm::{ClarityName, ContractName, Value};
use serde::{Deserialize, Deserializer};
use url::Url;

use crate::config::Settings;
use crate::error::Error;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The response from a GET /v2/data_var/<contract-principal>/<contract-name>/<var-name> request.
#[derive(Debug, Deserialize)]
pub struct DataVarResponse {
    /// The value of the data variable.
    #[serde(deserialize_with = "clarity_value_deserializer")]
    pub data: Value,
}

/// A client for interacting with Stacks nodes and the Stacks API
#[derive(Debug, Clone)]
pub struct StacksClient {
    /// The base url for the Stacks node's RPC API.
    pub endpoint: Url,
    /// The client used to make the request.
    pub client: reqwest::Client,
    /// The address of the deployer of the sBTC smart contracts.
    pub deployer: StacksAddress,
}

impl StacksClient {
    /// Create a new instance of the Stacks client using the given
    /// StacksSettings.
    pub fn new(url: Url, deployer: StacksAddress) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self {
            endpoint: url,
            client,
            deployer,
        })
    }

    /// Retrieve the latest value of a data variable from the specified contract.
    ///
    /// This is done by making a
    /// `GET /v2/data_var/<contract-principal>/<contract-name>/<var-name>`
    /// request. In the request we specify that the proof should not be included
    /// in the response.
    #[tracing::instrument(skip_all)]
    pub async fn get_data_var(
        &self,
        contract_principal: &StacksAddress,
        contract_name: &ContractName,
        var_name: &ClarityName,
    ) -> Result<Value, Error> {
        let path = format!("/v2/data_var/{contract_principal}/{contract_name}/{var_name}?proof=0",);

        let url = self
            .endpoint
            .join(&path)
            .map_err(|err| Error::PathJoin(err, self.endpoint.clone(), Cow::Owned(path)))?;

        tracing::debug!(
            %contract_principal,
            %contract_name,
            %var_name,
            "fetching contract data variable"
        );

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(Error::StacksNodeRequest)?;

        response
            .error_for_status()
            .map_err(Error::StacksNodeResponse)?
            .json::<DataVarResponse>()
            .await
            .map_err(Error::UnexpectedStacksResponse)
            .map(|x| x.data)
    }

    /// Retrieve the current signers' aggregate key from the `sbtc-registry`
    /// contract.
    pub async fn get_current_signers_aggregate_key(&self) -> Result<Option<PublicKey>, Error> {
        let value = self
            .get_data_var(
                &self.deployer,
                &ContractName::from("sbtc-registry"),
                &ClarityName::from("current-aggregate-pubkey"),
            )
            .await?;

        extract_aggregate_key(value)
    }
}

impl TryFrom<&Settings> for StacksClient {
    type Error = Error;

    fn try_from(value: &Settings) -> Result<Self, Self::Error> {
        let stacks_config = value
            .stacks
            .clone()
            .ok_or_else(|| Error::MissingStacksConfig)?;

        StacksClient::new(stacks_config.rpc_endpoint, stacks_config.deployer)
    }
}

/// A deserializer for Clarity's [`Value`] type that deserializes a hex-encoded
/// string which was serialized using Clarity's consensus serialization format.
fn clarity_value_deserializer<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    Value::try_deserialize_hex_untyped(&String::deserialize(deserializer)?)
        .map_err(serde::de::Error::custom)
}

/// Extract a aggregate key from a Clarity value.
///
/// In the sbtc-registry smart contract, the aggregate key is stored in the
/// `current-aggregate-pubkey` data var and is initialized to the 0x00
/// byte, allowing use to distinguish between the initial value and an
/// actual public key in that case. Ok(None) is returned if the value is
/// the initial value.
fn extract_aggregate_key(value: Value) -> Result<Option<PublicKey>, Error> {
    match value {
        Value::Sequence(SequenceData::Buffer(BuffData { data })) => {
            // The initial value of the data var is all zeros
            if data.as_slice() == [0u8] {
                Ok(None)
            } else {
                PublicKey::from_slice(&data)
                    .map(Some)
                    .map_err(Error::InvalidPublicKey)
            }
        }
        _ => Err(Error::InvalidStacksResponse(
            "expected a buffer but got something else",
        )),
    }
}

#[cfg(test)]
mod tests {
    use bitcoin::secp256k1::SECP256K1;
    use bitcoin::{NetworkKind, PrivateKey};
    use clarity::types::Address;
    use clarity::vm::types::{BuffData, SequenceData};
    use test_case::test_case;

    use super::*;

    /// Helper method for generating a list of public keys.
    fn generate_pubkeys(count: u16) -> Vec<PublicKey> {
        (0..count)
            .map(|_| {
                PublicKey::from_private_key(SECP256K1, &PrivateKey::generate(NetworkKind::Test))
            })
            .collect()
    }

    #[test_case(false; "some")]
    #[test_case(true; "none")]
    #[tokio::test]
    async fn get_current_signers_aggregate_key_works(return_none: bool) {
        let aggregate_key = generate_pubkeys(1)[0];

        let data;
        let expected;
        if return_none {
            // 0x00 is the initial value of the signers' aggregate key in
            // the sbtc-registry contract, and
            // get_current_signers_aggregate_key should return None when we
            // receive it.
            data = vec![0];
            expected = None;
        } else {
            data = aggregate_key.inner.serialize().to_vec();
            expected = Some(aggregate_key);
        }
        let aggregate_key_clarity = Value::Sequence(SequenceData::Buffer(BuffData { data }));

        // The format of the response JSON is `{"data": "0x<serialized-value>"}` (excluding the proof).
        let raw_json_response = format!(
            r#"{{"data":"0x{}"}}"#,
            Value::serialize_to_hex(&aggregate_key_clarity).expect("failed to serialize value")
        );

        // Setup our mock server
        let mut stacks_node_server = mockito::Server::new_async().await;
        let mock = stacks_node_server
            .mock("GET", "/v2/data_var/ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM/sbtc-registry/current-aggregate-pubkey?proof=0")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(&raw_json_response)
            .expect(1)
            .create();

        // Setup our Stacks client
        let client_url = url::Url::parse(stacks_node_server.url().as_str()).unwrap();
        let deployer =
            StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap();
        let client = StacksClient::new(client_url, deployer).unwrap();

        // Make the request to the mock server
        let resp = client.get_current_signers_aggregate_key().await.unwrap();

        // Assert that the response is what we expect
        assert_eq!(resp, expected);
        mock.assert();
    }
}
