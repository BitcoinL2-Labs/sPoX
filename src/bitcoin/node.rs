//! Contains client wrappers for bitcoin core

use std::sync::Arc;

use bitcoin::ScriptBuf;
use bitcoincore_rpc::{Auth, RpcApi};
use bitcoincore_rpc_json::{GetChainTipsResultStatus, ScanTxOutRequest, Utxo as RpcUtxo};
use url::Url;

use crate::bitcoin::{BlockRef, Utxo};
use crate::error::Error;

impl From<RpcUtxo> for Utxo {
    fn from(value: RpcUtxo) -> Self {
        Utxo {
            txid: value.txid,
            vout: value.vout,
            script_pub_key: value.script_pub_key,
            amount: value.amount,
            block_height: value.height,
        }
    }
}

/// A client for interacting with bitcoin-core
#[derive(Clone)]
pub struct BitcoinCoreClient {
    /// The underlying bitcoin-core client
    inner: Arc<bitcoincore_rpc::Client>,
}

/// Implement TryFrom for Url to allow for easy conversion from a URL to a
/// BitcoinCoreClient.
impl TryFrom<&Url> for BitcoinCoreClient {
    type Error = Error;

    fn try_from(url: &Url) -> Result<Self, Self::Error> {
        let username = url.username().to_string();
        let password = url.password().unwrap_or_default().to_string();
        let host = url
            .host_str()
            .ok_or(Error::InvalidUrl(url::ParseError::EmptyHost))?;
        let port = url.port().ok_or(Error::PortRequired)?;

        let endpoint = format!("{}://{host}:{port}", url.scheme());

        Self::new(&endpoint, username, password)
    }
}

impl BitcoinCoreClient {
    /// Return a bitcoin-core RPC client. Will error if the URL is an invalid URL.
    pub fn new(url: &str, username: String, password: String) -> Result<Self, Error> {
        let auth = Auth::UserPass(username, password);
        let client = bitcoincore_rpc::Client::new(url, auth)
            .map_err(|err| Error::BitcoinCoreRpcClient(err, url.to_string()))?;

        Ok(Self { inner: Arc::new(client) })
    }

    /// Get the canonical chain tip
    pub fn get_chain_tip(&self) -> Result<BlockRef, Error> {
        let result = self
            .inner
            .get_chain_tips()
            .map_err(Error::BitcoinCoreRpc)?
            .into_iter()
            .find(|tip| tip.status == GetChainTipsResultStatus::Active)
            .ok_or(Error::NoChainTip)?;

        Ok(BlockRef {
            block_hash: result.hash,
            block_height: result.height,
        })
    }

    /// Get UTXOs for addresses
    /// TODO: change `addresses` to an iterator
    pub fn get_utxos(&self, addresses: &[ScriptBuf]) -> Result<Vec<Utxo>, Error> {
        let descriptors = addresses
            .iter()
            .map(|addr| ScanTxOutRequest::Single(format!("raw({})", addr.to_hex_string())))
            .collect::<Vec<_>>();

        let result = self
            .inner
            .scan_tx_out_set_blocking(&descriptors)
            .map_err(Error::BitcoinCoreRpc)?;

        if result.success != Some(true) {
            return Err(Error::ScanTxOutFailure);
        }

        Ok(result.unspents.into_iter().map(Into::into).collect())
    }

    /// Get the canonical block hash for a given block height
    pub fn get_block_hash(&self, block_height: u64) -> Result<bitcoin::BlockHash, Error> {
        self.inner
            .get_block_hash(block_height)
            .map_err(Error::BitcoinCoreRpc)
    }

    /// Get the transaction hex
    pub fn get_raw_transaction_hex(
        &self,
        txid: &bitcoin::Txid,
        block_hash: &bitcoin::BlockHash,
    ) -> Result<String, Error> {
        self.inner
            .get_raw_transaction_hex(txid, Some(block_hash))
            .map_err(Error::BitcoinCoreRpc)
    }
}
