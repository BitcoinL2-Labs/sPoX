# sPoX

`sPoX` is a binary that monitors the Bitcoin blockchain for sBTC deposits made to a set of addresses, and when found
informs Emily about them so the sBTC signers can then process them.

Its primary usage is to enable automatic bridging of Stacks POX payments to sBTC.

## Building

To build `sPoX`, run:
```bash
cargo build --bin spox --release --locked
```

The binary will be built in `target/release/spox`.

## Configuration

You can specify which deposits to look for and the endpoints to use in a toml file.
See `src/config/default.toml` for a config starting point. Note that the Stacks configuration is required only to run
the `get-signers-xonly-key` command.

### Get signers xonly public key

When configuring a deposit, you must specify for what signers public key it is for in `signers_xonly`. This key changes
over time after sBTC key rotations. To fetch the current key, fill the `stacks` stanza with the Stacks endpoint and 
deployer address (for Stacks mainnet, see https://github.com/stacks-sbtc/sbtc/blob/main/docker/mainnet/sbtc-signer/signer-config.toml.in#L109).

Then you can run:
```bash
./spox -c <config file> get-signers-xonly-key
```
to get the latest key from the sBTC registry smart contract.

### Get a deposit address

Once you have configured a deposit, you can run:
```bash
./spox -c <config file> get-deposit-address
```
To get the deposit address, for each configured deposit.

## Run `sPoX`

Once the configuration is completed, you can run `sPoX`:
```bash
./spox -c <config file>
```
The binary will monitor the Bitcoin blockchain for payments made to the monitored addresses, and when a new payment is
confirmed, it will notify Emily about it so that the sBTC can process it.

## Devenv demo

`sPox` can be tested with the sBTC devenv:
 - `make devenv-up`, wait for nakamoto and `./signers.sh demo` to get the signers ready
 - Edit `signer/src/bin/demo_cli.rs`, `exec_deposit` to return after `send_raw_transaction` but before `create_deposit`
 
Now, in no particular order:
 - Start spox (overwriting the devenv aggregate key; or edit the config with the value returned from `get-signers-xonly-key`)
    ```bash
    SPOX_DEPOSIT__DEMO__SIGNERS_XONLY=$(RUST_LOG=info cargo run -- -c src/config/default.toml get-signers-xonly-key) RUST_LOG=debug cargo run -- -c src/config/default.toml
    ```
 - Create a deposit (without notifying emily): `cargo run -p signer --bin demo-cli deposit --amount 123456` (from sBTC)

This will look for deposits made to the signers pubkey with the devenv default values. Once the tx is confirmed it should appear on Emily, assuming it didn't expire in the meantime, and be processed by the signers, assuming the amount is not too low to be ignored.
