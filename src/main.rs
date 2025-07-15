use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use spox::config::Settings;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogOutputFormat {
    Json,
    Pretty,
}

/// Command line arguments
#[derive(Debug, Parser)]
#[clap(name = "sPoX")]
struct Args {
    /// Optional path to the configuration file. If not provided, it is expected
    /// that all required parameters are provided via environment variables.
    #[clap(short = 'c', long, required = false)]
    config: Option<PathBuf>,

    #[clap(short = 'o', long = "output-format", default_value = "pretty")]
    output_format: LogOutputFormat,
}

#[tokio::main]
#[tracing::instrument(name = "spox")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the command line arguments.
    let args = Args::parse();

    // Configure the binary's stdout/err output based on the provided output format.
    let pretty = matches!(args.output_format, LogOutputFormat::Pretty);
    spox::logging::setup_logging("info,spox=debug", pretty);

    // Load the configuration file and/or environment variables.
    let config = Settings::new(args.config).inspect_err(|error| {
        tracing::error!(%error, "failed to construct the configuration");
    })?;

    // TODO: remove, demo code
    let bitcoin_client =
        spox::bitcoin::node::BitcoinCoreClient::try_from(&config.bitcoin_rpc_endpoint)?;
    dbg!(bitcoin_client.get_chain_tip().unwrap());

    Ok(())
}
