# spox

`spox` is a binary that monitors the Bitcoin blockchain for sBTC deposits made to a set of addresses, and when found
informs Emily about them so the sBTC signers can then process them.

[sBTC](https://docs.stacks.co/concepts/sbtc) is a 1:1 BTC-backed asset on the Stacks blockchain: every sBTC token on Stacks corresponds to a real BTC deposit on
Bitcoin.

To mint sBTC, a user submits a Bitcoin transaction whose details the sBTC signers need to verify the deposit.
Those details are compactly encoded within the deposit transaction itself, and must be relayed via the
[Emily API](https://docs.stacks.co/concepts/sbtc/emily) before the signers can process a deposit.

The `spox` application uses the deposit-transaction data in its configuration file to monitor the Bitcoin network for
matching transactions and automatically notify Emily when a confirmed deposit appears.

## Building

To build `spox`, run:
```bash
cargo build --bin spox --release --locked
```

The binary will be built in `target/release/spox`.

## Configuration

You can specify which deposits to look for and the endpoints to use in a toml file.
See `src/config/default.toml` for a config starting point.

A Bitcoin node is required to run the binary (monitoring mode), while it is not used for specific CLI commands;
note that the entry in the config is still required (but not used).

A Stacks node is required only for the `get-signers-xonly-key` command, and can be omitted from the config if not used.

### Get signers xonly public key

When configuring a deposit, you must specify the sBTC signers' public key using the `signers_xonly` field in the config.
This key changes over time after sBTC key rotations. To fetch the current key, fill the `stacks` stanza with the Stacks
endpoint and deployer address (for Stacks mainnet, see https://github.com/stacks-sbtc/sbtc/blob/58669393deadfa2b786c34f7a575cdc3fcb58d0a/docker/mainnet/sbtc-signer/signer-config.toml.in#L109).

Then you can run:
```bash
./spox -c <config file> get-signers-xonly-key
```
to get the latest key from the sBTC registry smart contract. The config file will be searched for in the current working
directory, but it's also possible to specify an absolute path.

### Get a deposit address

Once you have configured a deposit, you can run:
```bash
./spox -c <config file> get-deposit-address
```
to get the bitcoin address for each configured deposit.

## Run `spox`

Once the configuration is completed, you can run `spox`:
```bash
./spox -c <config file>
```
The binary will monitor the Bitcoin blockchain for payments made to the monitored addresses, and when a new payment is
confirmed, it will notify Emily about it so that the sBTC signers can process it.

## Devenv demo

`spox` can be tested with the sBTC devenv:
 - `make devenv-up`, wait for nakamoto and `./signers.sh demo` to get the signers ready
 - Edit `signer/src/bin/demo_cli.rs`, `exec_deposit` to return after `send_raw_transaction` but before `create_deposit`
 
Now, in no particular order:
 - Start spox (overwriting the devenv aggregate key; or edit the config with the value returned from `get-signers-xonly-key`)
    ```bash
    SPOX_DEPOSIT__DEMO__SIGNERS_XONLY=$(RUST_LOG=info cargo run -- -c src/config/default.toml get-signers-xonly-key) RUST_LOG=debug cargo run -- -c src/config/default.toml
    ```
 - Create a deposit (without notifying emily): `cargo run -p signer --bin demo-cli deposit --amount 123456` (from sBTC)

This will look for deposits made to the signers pubkey with the devenv default values. Once the tx is confirmed it should appear on Emily, assuming it didn't expire in the meantime, and be processed by the signers, assuming the amount is not too low to be ignored.
