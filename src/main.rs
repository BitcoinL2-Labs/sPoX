use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use bitcoin::{ScriptBuf, secp256k1};
use clap::{Parser, Subcommand, ValueEnum};
use clarity::vm::types::PrincipalData;
use emily_client::apis::deposit_api;
use sbtc::deposits::{DepositScriptInputs, ReclaimScriptInputs};
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

#[derive(Debug, Subcommand)]
enum CliCommand {
    GetSignersPubkey,
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

    /// TODO: remove once config is ready
    #[clap(long = "signers-xonly")]
    signers_xonly: String,
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

fn devenv_deposit_address(signers_xonly: &str) -> MonitoredDeposit {
    MonitoredDeposit {
        deposit_script_inputs: DepositScriptInputs {
            signers_public_key: secp256k1::XOnlyPublicKey::from_str(signers_xonly).unwrap(),
            recipient: PrincipalData::parse("ST3497E9JFQ7KB9VEHAZRWYKF3296WQZEXBPXG193").unwrap(),
            max_fee: 20000,
        },
        reclaim_script_inputs: ReclaimScriptInputs::try_new(10, ScriptBuf::from_hex("").unwrap())
            .unwrap(),
    }
}

async fn get_signers_pubkey(config: &Settings) -> Result<(), Box<dyn std::error::Error>> {
    let stacks_client = StacksClient::try_from(config)?;

    let signers_aggregate_key = stacks_client.get_current_signers_aggregate_key().await?;

    match signers_aggregate_key {
        Some(public_key) => println!("{public_key}"),
        None => return Err(Box::new(Error::NoSignersAggregateKey)),
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

    let context = Context::try_from(&config)?;

    match args.command {
        Some(CliCommand::GetSignersPubkey) => return get_signers_pubkey(&config).await,
        None => (),
    }

    // TODO: load from config
    let monitored = vec![devenv_deposit_address(&args.signers_xonly)];

    let mut deposit_monitor = DepositMonitor::new(context.clone(), monitored);

    runloop(context, &mut deposit_monitor, config.polling_interval).await;

    Ok(())
}
