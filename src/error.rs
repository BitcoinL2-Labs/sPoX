//! Top-level error type

use bitcoin::ScriptBuf;

/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from the Bitcoin RPC client.
    #[error("bitcoin RPC error: {0}")]
    BitcoinCoreRpc(#[from] bitcoincore_rpc::Error),

    /// Error when creating an RPC client to bitcoin-core
    #[error("could not create RPC client to {1}: {0}")]
    BitcoinCoreRpcClient(#[source] bitcoincore_rpc::Error, String),

    /// The pending deposit is expired
    #[error("the pending deposit is expired")]
    DepositExpired,

    /// Error when parsing a URL
    #[error("could not parse the provided URL: {0}")]
    InvalidUrl(#[source] url::ParseError),

    /// No chain tip found.
    #[error("no bitcoin chain tip")]
    NoChainTip,

    /// Missing monitored deposit address for scriptPubKey
    #[error("missing monitored deposit address for scriptPubKey {0}")]
    MissingMonitoredDeposit(ScriptBuf),

    /// Error when the port is not provided
    #[error("a port must be specified")]
    PortRequired,

    /// A call to `scantxoutset` failed
    #[error("a call to `scantxoutset` failed")]
    ScanTxOutFailure,
}
