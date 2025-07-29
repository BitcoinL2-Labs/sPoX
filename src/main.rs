use std::path::PathBuf;
use std::time::Duration;

use bitcoin::Address;
use clap::{Parser, Subcommand, ValueEnum};
use emily_client::apis::deposit_api;
use spox::bitcoin::BlockRef;
use spox::config::Settings;
use spox::context::Context;
use spox::deposit_monitor::{DepositMonitor, MonitoredDeposit};
use spox::error::Error;
use spox::stacks::node::StacksClient;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogOutputFormat {
    Json,
    Pretty,
}

#[derive(Debug, Clone, Parser)]
struct GetDepositAddressArgs {
    #[clap(short = 'n', long = "network", default_value = "bitcoin")]
    pub network: bitcoin::Network,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    GetSignersXonlyKey,
    GetDepositAddress(GetDepositAddressArgs),
}

/// Command line arguments
#[derive(Debug, Parser)]
#[clap(name = "sPoX")]
struct Args {
    #[command(subcommand)]
    command: Option<CliCommand>,

    /// Optional path to the configuration file. If not provided, it is expected
    /// that all required parameters are provided via environment variables.
    #[clap(short = 'c', long, required = false)]
    config: Option<PathBuf>,

    #[clap(short = 'o', long = "output-format", default_value = "pretty")]
    output_format: LogOutputFormat,
}

async fn fetch_and_create_deposits(
    context: &Context,
    deposit_monitor: &mut DepositMonitor,
    chain_tip: &BlockRef,
) -> Result<(), Error> {
    let emily_config = context.emily_config();

    let deposits = deposit_monitor.get_pending_deposits(chain_tip)?;

    tracing::debug!(count = deposits.len(), "fetched pending deposits");
    if deposits.is_empty() {
        return Ok(());
    }

    for deposit in deposits {
        // TODO: emily will nop for duplicates, but we shouldn't send them
        if let Err(error) = deposit_api::create_deposit(emily_config, deposit.clone()).await {
            tracing::warn!(
                %error,
                txid = %deposit.bitcoin_txid,
                vout = %deposit.bitcoin_tx_output_index,
                "cannot create deposit in emily"
            );
        } else {
            tracing::info!(
                txid = %deposit.bitcoin_txid,
                vout = %deposit.bitcoin_tx_output_index,
                "created deposit in emily"
            );
        }
    }

    Ok(())
}

async fn runloop(
    context: Context,
    deposit_monitor: &mut DepositMonitor,
    polling_interval: Duration,
) {
    let bitcoin_client = context.bitcoin_client();
    let mut last_chain_tip = None;

    loop {
        if last_chain_tip.is_some() {
            tokio::time::sleep(polling_interval).await;
        }

        let chain_tip = match bitcoin_client.get_chain_tip() {
            Ok(chain_tip) => chain_tip,
            Err(error) => {
                tracing::warn!(
                    %error,
                    "error getting the chain tip"
                );
                continue;
            }
        };

        let is_last_chaintip = last_chain_tip
            .as_ref()
            .is_some_and(|last| last == &chain_tip);

        if is_last_chaintip {
            continue;
        }

        tracing::debug!(%chain_tip, "new block; processing pending deposits");

        let _ = fetch_and_create_deposits(&context, deposit_monitor, &chain_tip)
            .await
            .inspect_err(|error| {
                tracing::warn!(
                    %error,
                    "error processing pending deposits"
                )
            });

        last_chain_tip = Some(chain_tip);
    }
}

async fn get_signers_xonly_key(config: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let stacks_client = StacksClient::try_from(config)?;

    let signers_aggregate_key = stacks_client.get_current_signers_aggregate_key().await?;

    match signers_aggregate_key {
        Some(public_key) => println!("{public_key}"),
        None => return Err(Box::new(Error::NoSignersAggregateKey)),
    }

    Ok(())
}

async fn get_deposit_address(
    monitored: &[MonitoredDeposit],
    args: &GetDepositAddressArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    for deposit in monitored {
        let address = Address::from_script(&deposit.to_script_pubkey(), args.network)?;
        println!("{}: {}", deposit.alias, address);
    }
    Ok(())
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

    let monitored = config
        .deposit
        .iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, Error>>()?;

    match args.command {
        Some(CliCommand::GetSignersXonlyKey) => return get_signers_xonly_key(&config).await,
        Some(CliCommand::GetDepositAddress(args)) => {
            return get_deposit_address(&monitored, &args).await;
        }
        None => (),
    }

    let context = Context::try_from(&config)?;

    let mut deposit_monitor = DepositMonitor::new(context.clone(), monitored);

    runloop(context, &mut deposit_monitor, config.polling_interval).await;

    Ok(())
}
