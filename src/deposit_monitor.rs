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
    tx_hex_cache: LruCache<(Txid, BlockHash), String>,
}

// TODO: make cache size configurable
// As for now numbers are chosen to keep cache size around 4MB
const TX_HEX_CACHE_SIZE: NonZero<usize> =
    NonZero::new(8_000_usize).expect("Cache size must be non-zero");

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
            tx_hex_cache: LruCache::new(TX_HEX_CACHE_SIZE),
        }
    }

    /// Process a `Utxo` to get a create deposit request for Emily
    pub fn get_deposit_from_utxo(
        &mut self,
        utxo: &Utxo,
        chain_tip: &BlockRef,
    ) -> Result<CreateDepositRequestBody, Error> {
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

        let block_hash = bitcoin_client.get_block_hash(utxo.block_height)?;

        let tx_hex = self
            .tx_hex_cache
            .try_get_or_insert((utxo.txid, block_hash), || {
                bitcoin_client.get_raw_transaction_hex(&utxo.txid, &block_hash)
            })?
            .clone();

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
