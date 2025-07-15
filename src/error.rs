//! Top-level error type

/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from the Bitcoin RPC client.
    #[error("bitcoin RPC error: {0}")]
    BitcoinCoreRpc(#[from] bitcoincore_rpc::Error),

    /// Error when creating an RPC client to bitcoin-core
    #[error("could not create RPC client to {1}: {0}")]
    BitcoinCoreRpcClient(#[source] bitcoincore_rpc::Error, String),

    /// Error when parsing a URL
    #[error("could not parse the provided URL: {0}")]
    InvalidUrl(#[source] url::ParseError),

    /// No chain tip found.
    #[error("no bitcoin chain tip")]
    NoChainTip,

    /// Error when the port is not provided
    #[error("a port must be specified")]
    PortRequired,

    /// A call to `scantxoutset` failed
    #[error("a call to `scantxoutset` failed")]
    ScanTxOutFailure,
}
