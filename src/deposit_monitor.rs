//! Module to monitor for pending deposits

use std::collections::HashMap;
use std::num::NonZero;

use bitcoin::{BlockHash, ScriptBuf, Txid};
use emily_client::models::CreateDepositRequestBody;
use lru::LruCache;
use sbtc::deposits::{DepositScriptInputs, ReclaimScriptInputs};

use crate::bitcoin::{BlockRef, Utxo};
use crate::context::Context;
use crate::error::Error;

/// A deposit address to monitor
pub struct MonitoredDeposit {
    /// Deposit script inputs
    pub deposit_script_inputs: DepositScriptInputs,
    /// Reclaim script inputs
    pub reclaim_script_inputs: ReclaimScriptInputs,
}

impl MonitoredDeposit {
    /// Get the scriptPubKey for this deposit address
    pub fn to_script_pubkey(&self) -> ScriptBuf {
        sbtc::deposits::to_script_pubkey(
            self.deposit_script_inputs.deposit_script(),
            self.reclaim_script_inputs.reclaim_script(),
        )
    }
}

/// Deposit monitor
pub struct DepositMonitor {
    context: Context,
    monitored: HashMap<ScriptBuf, MonitoredDeposit>,
    hashes_cache: LruCache<u64, BlockHash>,
    tx_hex_cache: LruCache<(Txid, BlockHash), String>,
}

// TODO: make cache sizes configurable
// As for now numbers are chosen to keep cache size around 10MB
const HASHES_CACHE_SIZE: NonZero<usize> = NonZero::new(260_000_usize).unwrap();
const TX_HEX_CACHE_SIZE: NonZero<usize> = NonZero::new(20_000_usize).unwrap();

impl DepositMonitor {
    /// Creates a new `DepositMonitor`
    pub fn new(context: Context, monitored: Vec<MonitoredDeposit>) -> Self {
        let monitored = monitored
            .into_iter()
            .map(|m| (m.to_script_pubkey(), m))
            .collect();

        Self {
            context,
            monitored,
            hashes_cache: LruCache::new(HASHES_CACHE_SIZE),
            tx_hex_cache: LruCache::new(TX_HEX_CACHE_SIZE),
        }
    }

    /// Process a `Utxo` to get a create deposit request for Emily
    pub fn get_deposit_from_utxo(
        &mut self,
        utxo: &Utxo,
        chain_tip: &BlockRef,
    ) -> Result<CreateDepositRequestBody, Error> {
        tracing::debug!(
            "Processing utxo: txid={}, vout={}, block_height={}",
            utxo.txid,
            utxo.vout,
            utxo.block_height
        );
        let monitored_deposit = self
            .monitored
            .get(&utxo.script_pub_key)
            .ok_or_else(|| Error::MissingMonitoredDeposit(utxo.script_pub_key.clone()))?;

        let unlocking_time =
            utxo.block_height + (monitored_deposit.reclaim_script_inputs.lock_time() as u64);
        if unlocking_time <= chain_tip.block_height {
            return Err(Error::DepositExpired);
        }

        let bitcoin_client = self.context.bitcoin_client();

        let cached_hash = self.hashes_cache.get(&utxo.block_height);
        let block_hash = match cached_hash {
            Some(hash) => {
                *hash
            }
            None => {
                let hash = bitcoin_client.get_block_hash(utxo.block_height)?;
                self.hashes_cache.put(utxo.block_height, hash);
                hash
            }
        };

        let cached_tx_hex = self.tx_hex_cache.get(&(utxo.txid, block_hash));
        let tx_hex = match cached_tx_hex {
            Some(hex) => {
                hex.clone()
            }
            None => {
                let tx_hex = bitcoin_client.get_raw_transaction_hex(&utxo.txid, &block_hash)?;
                self.tx_hex_cache
                    .put((utxo.txid, block_hash), tx_hex.clone());
                tx_hex
            }
        };

        Ok(CreateDepositRequestBody {
            bitcoin_tx_output_index: utxo.vout,
            bitcoin_txid: utxo.txid.to_string(),
            deposit_script: monitored_deposit
                .deposit_script_inputs
                .deposit_script()
                .to_hex_string(),
            reclaim_script: monitored_deposit
                .reclaim_script_inputs
                .reclaim_script()
                .to_hex_string(),
            transaction_hex: tx_hex,
        })
    }

    /// Check pending deposits confirmed to the monitored addresses
    pub fn get_pending_deposits(
        &mut self,
        chain_tip: &BlockRef,
    ) -> Result<Vec<CreateDepositRequestBody>, Error> {
        tracing::debug!(
            "Checking for pending deposits at chain tip: {}",
            chain_tip.block_height
        );
        let utxos = self
            .context
            .bitcoin_client()
            .get_utxos(self.monitored.keys())?;

        let create_deposits = utxos
            .iter()
            .flat_map(|utxo| {
                self.get_deposit_from_utxo(utxo, chain_tip)
                    .inspect_err(|error| match error {
                        Error::DepositExpired => tracing::info!(
                            %error,
                            txid = %utxo.txid,
                            vout = %utxo.vout,
                            block_height = %utxo.block_height,
                            "deposit is expired; skipping utxo"
                        ),
                        _ => tracing::warn!(
                            %error,
                            txid = %utxo.txid,
                            vout = %utxo.vout,
                            block_height = %utxo.block_height,
                            "failed to get deposit from utxo; skipping utxo"
                        ),
                    })
                    .ok()
            })
            .collect();

        Ok(create_deposits)
    }
}
