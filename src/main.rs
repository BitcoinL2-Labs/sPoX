use std::path::PathBuf;

use clap::Parser;
use spox::config::Settings;

/// Command line arguments
#[derive(Debug, Parser)]
#[clap(name = "sPoX")]
struct Args {
    /// Optional path to the configuration file. If not provided, it is expected
    /// that all required parameters are provided via environment variables.
    #[clap(short = 'c', long, required = false)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse the command line arguments.
    let args = Args::parse();

    // Load the configuration file and/or environment variables.
    let config = Settings::new(args.config).inspect_err(|error| {
        tracing::error!(%error, "failed to construct the configuration");
    })?;

    dbg!(config);

    Ok(())
}
